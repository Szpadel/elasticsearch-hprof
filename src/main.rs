mod elasticsearch;
mod hprof;

use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::*;
use clap::{Args, Parser, Subcommand};
use elasticsearch::*;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[clap(alias = "inflight_queries")]
    InflightQueries(InflightQueries),
}

#[derive(Debug, Args)]
#[command(about = "Read queries that was inflight in the time of crash\n\
    At least one of --print or --save is required")]
struct InflightQueries {
    #[arg(
        required_unless_present("save"),
        long,
        help = "Print queries to console"
    )]
    print: bool,
    #[arg(
        required_unless_present("print"),
        long,
        help = "Save queries to files, one per query, directory named <hprof_filename>.prof will be created"
    )]
    save: bool,
    #[arg(help = "Location of .hprof file from elasticsearch OOM dump")]
    hprof: PathBuf,
}

fn main() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    let cli = Cli::parse();

    match &cli.commands {
        Commands::InflightQueries(inflight) => {
            if let Err(err) = inflight_queries(inflight) {
                eprintln!("ERROR: {err:#}");
            }
        }
    }

    // if let Some((name, subcommand)) = matches.subcommand() {
    //     match name {
    //         "inflight_queries" => {
    //             let file_path = subcommand.value_of_os("hprof").unwrap();
    //             if let Err(err) = inflight_queries(InflightQueriesOpts {
    //                 save: subcommand.is_present("save"),
    //                 print: subcommand.is_present("print"),
    //                 hprof_file: Path::new(file_path).into(),
    //             }) {
    //                 eprintln!("ERROR: {:#}", err);
    //             }
    //         }
    //         _ => {}
    //     }
    // }
}

fn open_hprof_file(path: &PathBuf) -> Result<std::fs::File> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("Failed to access file at {:?}", path))?;

    if metadata.len() > 0 {
        return std::fs::File::open(path)
            .with_context(|| format!("Failed to open file at {:?}", path));
    }
    log::info!(
            "File at {:?} is empty, waiting for content...",
            path
        );

    let start_time = Instant::now();
    let mut last_size = 0;

    loop {
        thread::sleep(Duration::from_secs(30));

        let metadata = std::fs::metadata(path)
            .with_context(|| format!("Failed to access file at {:?}", path))?;
        let current_size = metadata.len();
        let elapsed = start_time.elapsed();

        if current_size == 0 && elapsed > Duration::from_secs(15 * 60) {
            return Err(anyhow::anyhow!(
                "Timeout after {:?} waiting for file to have content",
                elapsed
            ));
        }
        if current_size != last_size {
            log::info!("File size changed: {} -> {} bytes", last_size, current_size);
            last_size = current_size;
            continue;
        }
        if current_size > 0{
            return std::fs::File::open(path)
                .with_context(|| format!("Failed to open file at {:?}", path));
        }
    }
}

fn inflight_queries(opts: &InflightQueries) -> Result<()> {
    let file = open_hprof_file(&opts.hprof)?;
    let memmap = unsafe { memmap::MmapOptions::new().map(&file) }.context("Failed to mmap file")?;
    log::info!("Loading hprof file...");
    let elastic = ElasticsearchMemory::new(&memmap);
    log::info!("Extracting inflight queries...");
    let results_path = if opts.save {
        let mut results_path = opts.hprof.canonicalize().context("Failed to locate file")?;
        let mut filename = results_path
            .file_name()
            .context("Failed to prepare results dir")?
            .to_os_string();
        results_path.pop();
        filename.push(".prof");
        results_path.push(filename);
        if !results_path.exists() {
            std::fs::create_dir(&results_path).context("Failed to create results directory")?;
        }
        Some(results_path)
    } else {
        None
    };
    for (i, query) in elastic.read_inflight_queries().iter().enumerate() {
        eprintln!("query {i}");
        if opts.print {
            println!("{query}");
            println!();
        }
        if let Some(results_path) = &results_path {
            let mut query_filename = results_path.clone();
            query_filename.push(format!("query_{i}.json"));
            std::fs::write(query_filename, query).context("Failed to save query file")?;
        }
    }
    Ok(())
}
