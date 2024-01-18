use jvm_hprof::heap_dump::{Class, FieldDescriptors};

use super::{instance::JavaInstance, ClassId, JavaProfile};

pub struct JavaClass<'a> {
    class: Class<'a>,
}

impl<'a> JavaClass<'a> {
    pub fn new(class: Class<'a>) -> Self {
        Self { class }
    }

    pub fn name(&self, profile: &'a JavaProfile) -> &'a str {
        profile
            .load_classes
            .get(&self.class.obj_id().into())
            .and_then(|lc| profile.strings.get(&lc.class_name_id().into()))
            .unwrap()
    }

    pub fn id(&self) -> ClassId {
        self.class.obj_id().into()
    }

    pub fn instances(&self, profile: &'a JavaProfile) -> Vec<&JavaInstance<'a>> {
        profile
            .class_instance_map
            .get(&self.id())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|id| {
                        profile.objects.get(id).and_then(|object| {
                            if let super::Object::Instance(i) = object {
                                Some(i)
                            } else {
                                None
                            }
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    pub fn instance_field_descriptors(&self) -> FieldDescriptors {
        self.class.instance_field_descriptors()
    }

    pub fn parent_class(&self) -> Option<ClassId> {
        self.class.super_class_obj_id().map(ClassId::from)
    }
}
