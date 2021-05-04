mod elasticsearch;
mod hprof;

use std::path::{Path, PathBuf};

use anyhow::*;
use clap::{App, AppSettings, Arg};
use elasticsearch::*;

struct InflightQueriesOpts {
    print: bool,
    save: bool,
    hprof_file: PathBuf,
}

fn main() {
    let matches = clap::app_from_crate!()
        .setting(AppSettings::SubcommandRequired)
        .subcommand(
            App::new("inflight_queries")
                .about(
                    "Read queries that was inflight in the time of crash\n\
                        At least one of --print or --save is required",
                )
                .long_about("At least one of --print or --save is required")
                .arg(
                    Arg::new("print")
                        .long("print")
                        .conflicts_with("save")
                        .required_unless_present("save")
                        .about("Print queries to terminal"),
                )
                .arg(
                    Arg::new("save")
                        .long("save")
                        .conflicts_with("print")
                        .required_unless_present("print")
                        .about("Save queries to files"),
                )
                .arg(
                    Arg::new("hprof")
                        .required(true)
                        .about("Location of .hprof file from elasticsearch OOM dump"),
                ),
        )
        .get_matches();

    if let Some((name, subcommand)) = matches.subcommand() {
        match name {
            "inflight_queries" => {
                let file_path = subcommand.value_of_os("hprof").unwrap();
                if let Err(err) = inflight_queries(InflightQueriesOpts {
                    save: subcommand.is_present("save"),
                    print: subcommand.is_present("print"),
                    hprof_file: Path::new(file_path).into(),
                }) {
                    eprintln!("ERROR: {:#}", err);
                }
            }
            _ => {}
        }
    }
}

fn inflight_queries(opts: InflightQueriesOpts) -> Result<()> {
    let file = std::fs::File::open(opts.hprof_file.clone()).context("Failed open hprof file")?;
    let memmap = unsafe { memmap::MmapOptions::new().map(&file) }.context("Failed to mmap file")?;
    eprintln!("Loading hprof file...");
    let elastic = ElasticsearchMemory::new(&memmap);
    eprintln!("Extracting inflight queries...");
    let results_path = if opts.save {
        let mut results_path = opts
            .hprof_file
            .canonicalize()
            .context("Failed to locate file")?;
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
        eprintln!("query {}", i);
        if opts.print {
            println!("{}", query);
            println!();
        }
        if let Some(results_path) = &results_path {
            let mut query_filename = results_path.clone();
            query_filename.push(format!("query_{}.json", i));
            std::fs::write(query_filename, query).context("Failed to save query file")?;
        }
    }
    Ok(())
}
