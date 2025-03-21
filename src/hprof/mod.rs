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
    class_id_index: AHashMap<String, ClassId>,
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
            class_id_index: Default::default(),
            objects: Default::default(),
            class_instance_map: Default::default(),
        }
    }

    pub fn process(&mut self) {
        log::trace!("Starting to process HPROF records");
        let mut record_num = 1;
        for record in self.hprof.records_iter().flatten() {
            record_num += 1;
            if record_num % 1000 == 0 {
                log::debug!("Processing record {}", record_num);
            }
            match record.tag() {
                jvm_hprof::RecordTag::LoadClass => {
                    if let Some(Ok(lc)) = record.as_load_class() {
                        log::trace!("Processing LoadClass: class_obj_id={:?}", lc.class_obj_id());
                        self.load_classes.insert(lc.class_obj_id().into(), lc);
                    }
                }
                jvm_hprof::RecordTag::Utf8 => {
                    if let Some(Ok(string)) = record.as_utf_8() {
                        let text = string.text_as_str().unwrap_or("(invalid UTF-8)");
                        // log::trace!("Processing UTF8 string: id={:?}, text={}", string.name_id(), text);
                        self.strings.insert(string.name_id().into(), text);
                    }
                }
                jvm_hprof::RecordTag::HeapDump | jvm_hprof::RecordTag::HeapDumpSegment => {
                    log::trace!("Processing heap dump segment");
                    if let Some(Ok(heap)) = record.as_heap_dump_segment() {
                        for sub in heap.sub_records().flatten() {
                            match sub {
                                jvm_hprof::heap_dump::SubRecord::Class(c) => {
                                    log::trace!("Processing class: obj_id={:?}", c.obj_id());
                                    self.classes.insert(c.obj_id().into(), JavaClass::new(c));
                                }
                                jvm_hprof::heap_dump::SubRecord::Instance(instance) => {
                                    // log::trace!("Processing instance: obj_id={:?}, class={:?}",
                                    //               instance.obj_id(), instance.class_obj_id());
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
                                    // log::trace!("Processing object array: obj_id={:?}",
                                    //               obj_array.obj_id());
                                    let instance = JavaObjectArray::new(obj_array);
                                    self.objects.insert(instance.id(), Object::Array(instance));
                                }
                                jvm_hprof::heap_dump::SubRecord::PrimitiveArray(pa) => {
                                    // log::trace!("Processing primitive array: obj_id={:?}",
                                    //               pa.obj_id());
                                    let instance = JavaPrimitiveArray::new(pa);
                                    self.objects
                                        .insert(instance.id(), Object::PrimitiveArray(instance));
                                }
                                _ => {
                                    log::trace!("Skipping unsupported heap dump sub-record type");
                                }
                            }
                        }
                    }
                }
                _ => {
                    log::trace!("Skipping unsupported record type: {:?}", record.tag());
                }
            }
        }
        log::trace!("Building class index");
        self.build_index();
        log::trace!(
            "Processing complete: {} classes, {} objects",
            self.classes.len(),
            self.objects.len()
        );
    }

    fn build_index(&mut self) {
        let mut class_id_index = AHashMap::new();
        for (id, class) in self.classes() {
            class_id_index.insert(class.name(self).to_string(), *id);
        }
        self.class_id_index = class_id_index;
    }

    pub fn classes(&self) -> hash_map::Iter<ClassId, JavaClass> {
        self.classes.iter()
    }

    pub fn get_class_by_name(&self, class_name: &str) -> Option<&JavaClass> {
        self.class_id_index
            .get(class_name)
            .and_then(|class_id| self.get_class_by_id(class_id))
    }

    pub fn get_class_by_id(&self, class_id: &ClassId) -> Option<&JavaClass> {
        self.classes.get(class_id)
    }

    /// Checks if the class with the given `child_id` is a subclass of the class with the given `parent_id`.\
    /// Returns `None` if either class is not found.
    pub fn is_subclass(&self, child_id: ClassId, parent_id: ClassId) -> Option<bool> {
        let mut class_id = Some(child_id);
        while class_id != Some(parent_id) && class_id.is_some() {
            let class = self.get_class_by_id(&class_id?)?;
            class_id = class.parent_class();
        }

        Some(class_id == Some(parent_id))
    }

    pub fn is_subclass_by_name(&self, child_name: &str, parent_name: &str) -> Option<bool> {
        let child_id = self.get_class_by_name(child_name)?.id();
        let parent_id = self.get_class_by_name(parent_name)?.id();
        self.is_subclass(child_id, parent_id)
    }
}
