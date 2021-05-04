mod class;
mod field_value;
mod ids;
mod instance;
mod object_array;
mod primitive_array;

use std::collections::hash_map::{self, Entry};

use ahash::AHashMap;
pub use class::*;
pub use field_value::*;
pub use ids::*;
pub use instance::*;
use jvm_hprof::{parse_hprof, Hprof, LoadClass};
pub use object_array::*;
pub use primitive_array::*;

pub enum Object<'a> {
    Instance(JavaInstance<'a>),
    Array(JavaObjectArray<'a>),
    PrimitiveArray(JavaPrimitiveArray<'a>),
}

pub struct JavaProfile<'a> {
    hprof: Hprof<'a>,
    load_classes: AHashMap<ClassId, LoadClass>,
    strings: AHashMap<StringId, &'a str>,
    classes: AHashMap<ClassId, JavaClass<'a>>,
    objects: AHashMap<ObjectId, Object<'a>>,
    class_instance_map: AHashMap<ClassId, Vec<ObjectId>>,
}

impl<'a> JavaProfile<'a> {
    pub fn new(mmap: &'a [u8]) -> Self {
        let hprof = parse_hprof(mmap).unwrap();
        Self {
            hprof,
            load_classes: Default::default(),
            strings: Default::default(),
            classes: Default::default(),
            objects: Default::default(),
            class_instance_map: Default::default(),
        }
    }

    pub fn process(&mut self) {
        for record in self.hprof.records_iter().flatten() {
            match record.tag() {
                jvm_hprof::RecordTag::LoadClass => {
                    if let Some(Ok(lc)) = record.as_load_class() {
                        self.load_classes.insert(lc.class_obj_id().into(), lc);
                    }
                }
                jvm_hprof::RecordTag::Utf8 => {
                    if let Some(Ok(string)) = record.as_utf_8() {
                        self.strings.insert(
                            string.name_id().into(),
                            string.text_as_str().unwrap_or("(invalid UTF-8)"),
                        );
                    }
                }
                jvm_hprof::RecordTag::HeapDump | jvm_hprof::RecordTag::HeapDumpSegment => {
                    if let Some(Ok(heap)) = record.as_heap_dump_segment() {
                        for sub in heap.sub_records().flatten() {
                            match sub {
                                jvm_hprof::heap_dump::SubRecord::Class(c) => {
                                    self.classes.insert(c.obj_id().into(), JavaClass::new(c));
                                }
                                jvm_hprof::heap_dump::SubRecord::Instance(instance) => {
                                    match self
                                        .class_instance_map
                                        .entry(instance.class_obj_id().into())
                                    {
                                        Entry::Occupied(mut entry) => {
                                            entry.get_mut().push(instance.obj_id().into());
                                        }
                                        Entry::Vacant(entry) => {
                                            entry.insert(vec![instance.obj_id().into()]);
                                        }
                                    }
                                    let instance = JavaInstance::new(instance);
                                    self.objects
                                        .insert(instance.id(), Object::Instance(instance));
                                }
                                jvm_hprof::heap_dump::SubRecord::ObjectArray(obj_array) => {
                                    let instance = JavaObjectArray::new(obj_array);
                                    self.objects.insert(instance.id(), Object::Array(instance));
                                }
                                jvm_hprof::heap_dump::SubRecord::PrimitiveArray(pa) => {
                                    let instance = JavaPrimitiveArray::new(pa);
                                    self.objects
                                        .insert(instance.id(), Object::PrimitiveArray(instance));
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn classes(&self) -> hash_map::Iter<ClassId, JavaClass> {
        self.classes.iter()
    }
}
