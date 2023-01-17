use std::borrow::Borrow;

use ahash::AHashMap;

use crate::hprof::*;

pub struct ElasticsearchMemory<'a> {
    profile: JavaProfile<'a>,
}

impl<'a> ElasticsearchMemory<'a> {
    pub fn new(mmap: &'a [u8]) -> Self {
        let mut profile = JavaProfile::new(mmap);
        profile.process();
        Self { profile }
    }

    pub fn read_inflight_queries(&self) -> Vec<String> {
        let mut queries = Vec::new();
        if let Some((_, class)) = self.profile.classes().find(|(_id, class)| {
            class.name(&self.profile) == "org/elasticsearch/http/netty4/Netty4HttpRequest"
        }) {
            for instance in class.instances(&self.profile) {
                if let Some(content_value) = instance
                    .local_fields(&self.profile)
                    .find(|f| f.name() == "content")
                {
                    if let JavaLocalValue::Object(content) = content_value.value(&self.profile) {
                        if let Some(query) = self.read_composite_bytes(content) {
                            queries.push(query);
                        }
                    }
                }
            }
        }

        queries
    }

    fn read_composite_chunk(&self, item: &JavaInstance, buffer: &mut String) -> Result<(), ()> {
        let fields: AHashMap<_, _> = item.local_fields(&self.profile).collect();

        let bytes = fields.get("bytes").and_then(|b| {
            if let JavaLocalValue::PrimitiveArray(arr) = b.value(&self.profile) {
                Some(arr)
            } else {
                None
            }
        });
        let offset = fields.get("offset").and_then(|o| {
            if let JavaLocalValue::Int(o) = o.value(&self.profile) {
                Some(o)
            } else {
                None
            }
        });
        let length = fields.get("length").and_then(|l| {
            if let JavaLocalValue::Int(l) = l.value(&self.profile) {
                Some(l)
            } else {
                None
            }
        });

        if let (Some(bytes), Some(offset), Some(length)) = (bytes, offset, length) {
            if let PrimitiveArrayValues::Byte(bytes) = bytes.values() {
                let offset = offset as usize;
                let length = length as usize;
                let parsed = bytes[offset..offset + length]
                    .iter()
                    .map(|&i| i as u8)
                    .collect::<Vec<_>>();
                let str = String::from_utf8_lossy(&parsed);
                buffer.push_str(str.borrow());
                return Ok(());
            }
        }
        Err(())
    }

    fn read_composite_bytes(&self, instance: &JavaInstance) -> Option<String> {
        if let Some(refs_value) = instance
            .local_fields(&self.profile)
            .find(|f| f.name() == "references")
        {
            if let JavaLocalValue::ObjectArray(refs) = refs_value.value(&self.profile) {
                let mut buffer = String::new();
                for item in refs.values(&self.profile).flatten() {
                    self.read_composite_chunk(&item, &mut buffer).ok();
                }
                return Some(buffer);
            }
        } else {
            let mut buffer = String::new();
            if let Ok(_) = self.read_composite_chunk(&instance, &mut buffer) {
                return Some(buffer);
            }
        }
        None
    }
}
