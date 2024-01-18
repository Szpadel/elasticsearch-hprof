use anyhow::{anyhow, Context};

use crate::hprof::*;

const HTTP_REQUEST_CLASS: &str = "org/elasticsearch/http/netty4/Netty4HttpRequest";
const COMPOSITE_BYTES_REFERENCE_CLASS: &str =
    "org/elasticsearch/common/bytes/CompositeBytesReference";
const BYTES_ARRAY_CLASS: &str = "org/elasticsearch/common/bytes/BytesArray";

pub struct ElasticsearchMemory<'a> {
    profile: JavaProfile<'a>,
}

impl<'a> ElasticsearchMemory<'a> {
    pub fn new(mmap: &'a [u8]) -> Self {
        let mut profile = JavaProfile::new(mmap);
        log::debug!("Processing profile...");
        profile.process();
        Self { profile }
    }

    pub fn read_inflight_queries(&self) -> Vec<String> {
        let mut queries = Vec::new();
        if let Some(class) = self.profile.get_class_by_name(HTTP_REQUEST_CLASS) {
            for http_request in class.instances(&self.profile) {
                log::debug!("Located HttpRequest {}", http_request.id());
                if let Some(i) = http_request
                    .fields(&self.profile)
                    .value::<&JavaInstance>(&self.profile, "released")
                {
                    if i.fields(&self.profile).value(&self.profile, "value") == Some(1i32) {
                        log::debug!("Request released, skipping");
                        continue;
                    }
                }
                self.debug_instance(http_request);
                match self.read_request_data(http_request) {
                    Ok(query) => {
                        if query.is_empty() {
                            log::warn!("Extracted query is empty, skipping");
                        } else {
                            queries.push(query);
                        }
                    }
                    Err(err) => {
                        log::error!("Failed to read query from request: {:#}", err)
                    }
                }
            }
        }

        queries
    }

    fn read_request_data(&self, http_request: &'a JavaInstance) -> anyhow::Result<String> {
        let fields = http_request.fields(&self.profile);
        let content: &JavaInstance = fields
            .value(&self.profile, "content")
            .ok_or(anyhow!("content not found"))?;

        match content
            .class(&self.profile)
            .map(|c| c.name(&self.profile))
            .unwrap_or("unknown")
        {
            COMPOSITE_BYTES_REFERENCE_CLASS => self.read_composite_bytes(content),
            BYTES_ARRAY_CLASS => self.read_bytes_array(content),
            class_name => Err(anyhow!("Unknown content class {class_name}")),
        }
    }

    fn read_bytes_array(&self, instance: &'a JavaInstance) -> anyhow::Result<String> {
        self.debug_instance(instance);
        let fields = instance.fields(&self.profile);
        let bytes: &JavaPrimitiveArray = fields
            .value(&self.profile, "bytes")
            .ok_or(anyhow!("bytes not found"))?;

        let offset: i32 = fields
            .value(&self.profile, "offset")
            .ok_or(anyhow!("offset not found"))?;

        let length: i32 = fields
            .value(&self.profile, "length")
            .ok_or(anyhow!("length not found"))?;

        if let PrimitiveArrayValues::Byte(bytes) = bytes.values() {
            let offset = offset as usize;
            let length = length as usize;
            let parsed = bytes[offset..offset + length]
                .iter()
                .map(|&i| i as u8)
                .collect::<Vec<_>>();
            let str = String::from_utf8_lossy(&parsed);
            Ok(str.to_string())
        } else {
            Err(anyhow!("Expected array of bytes"))
        }
    }

    fn read_composite_bytes(&self, composite_bytes: &JavaInstance) -> anyhow::Result<String> {
        self.debug_instance(composite_bytes);
        let fields = composite_bytes.fields(&self.profile);
        let references: &JavaObjectArray = fields
            .value(&self.profile, "references")
            .ok_or(anyhow!("failed to read references"))?;

        let references_class_name = references.class_name(&self.profile).unwrap_or("unknown");
        if self
            .profile
            .is_subclass_by_name(references_class_name, BYTES_ARRAY_CLASS)
            .unwrap_or(false)
        {
            log::warn!("Expected {BYTES_ARRAY_CLASS}, found {references_class_name}. Trying anyways, but it'll likely not succeed");
        }

        let query = references
            .values(&self.profile)
            .filter_map(|i| {
                if let Some(i) = i {
                    Some(
                        self.read_bytes_array(i)
                            .context("Failed read bytes array instance"),
                    )
                } else {
                    log::warn!("Could not read chunk request fragment! Expect corrupted query");
                    None
                }
            })
            .collect::<anyhow::Result<Vec<String>>>()?
            .join("");

        Ok(query)
    }

    fn debug_instance(&self, instance: &JavaInstance) {
        if log::log_enabled!(log::Level::Trace) {
            log::trace!(
                "Instance {} {} fields:",
                instance.name(&self.profile).unwrap_or("unknown"),
                instance.id()
            );
            let fields = instance.fields(&self.profile);
            for (name, field) in fields.fields {
                log::trace!(
                    "{name}: {}",
                    field.value(&self.profile).type_name(&self.profile)
                );
            }
        }
    }
}
