use std::iter::FromIterator;

use ahash::AHashMap;
use jvm_hprof::heap_dump::{FieldDescriptors, Instance};

use super::{JavaFieldValue, JavaProfile, ObjectId};

pub struct LocalFieldsIterator<'a> {
    fields_memory: &'a [u8],
    fd_iter: Option<FieldDescriptors<'a>>,
    profile: &'a JavaProfile<'a>,
}

impl<'a> LocalFieldsIterator<'a> {
    fn new(profile: &'a JavaProfile<'a>, instance: &'a Instance<'a>) -> Self {
        let class = profile.classes.get(&instance.class_obj_id().into());
        Self {
            fields_memory: instance.fields(),
            fd_iter: class.map(|c| c.instance_field_descriptors()),
            profile,
        }
    }
}

impl<'a> Iterator for LocalFieldsIterator<'a> {
    type Item = JavaFieldValue<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(Ok(fd)) = self.fd_iter.as_mut().and_then(|fd| fd.next()) {
            if let Ok((rest_memory, value)) = fd
                .field_type()
                .parse_value(self.fields_memory, self.profile.hprof.header().id_size())
            {
                self.fields_memory = rest_memory;
                if let Some(&field_name) = self.profile.strings.get(&fd.name_id().into()) {
                    return Some(JavaFieldValue::new(field_name, value));
                }
            }
        }
        None
    }
}

impl<'a> FromIterator<JavaFieldValue<'a>> for AHashMap<&'a str, JavaFieldValue<'a>> {
    fn from_iter<T: IntoIterator<Item = JavaFieldValue<'a>>>(iter: T) -> Self {
        let mut fields = AHashMap::default();
        for item in iter {
            fields.insert(item.name(), item);
        }
        fields
    }
}

pub struct JavaInstance<'a> {
    instance: Instance<'a>,
}

impl<'a> JavaInstance<'a> {
    pub fn new(instance: Instance<'a>) -> Self {
        Self { instance }
    }

    pub fn id(&self) -> ObjectId {
        self.instance.obj_id().into()
    }

    pub fn local_fields(&'a self, profile: &'a JavaProfile) -> LocalFieldsIterator<'a> {
        LocalFieldsIterator::new(profile, &self.instance)
    }
}
