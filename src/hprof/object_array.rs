use jvm_hprof::heap_dump::{NullableIds, ObjectArray};

use super::{ClassId, JavaInstance, JavaProfile, Object, ObjectId};

pub struct JavaObjectArrayIterator<'a> {
    profile: &'a JavaProfile<'a>,
    iter: NullableIds<'a>,
}

impl<'a> JavaObjectArrayIterator<'a> {
    fn new(profile: &'a JavaProfile<'a>, array: &'a ObjectArray<'a>) -> Self {
        Self {
            iter: array.elements(profile.hprof.header().id_size()),
            profile,
        }
    }
}

impl<'a> Iterator for JavaObjectArrayIterator<'a> {
    type Item = Option<&'a JavaInstance<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(Ok(item_id)) = self.iter.next() {
            return item_id.map(|id| match self.profile.objects.get(&id.into()) {
                Some(Object::Instance(instance)) => Some(instance),
                _ => None,
            });
        }
        None
    }
}

pub struct JavaObjectArray<'a> {
    array: ObjectArray<'a>,
}

impl<'a> JavaObjectArray<'a> {
    pub fn new(array: ObjectArray<'a>) -> Self {
        Self { array }
    }

    pub fn id(&self) -> ObjectId {
        self.array.obj_id().into()
    }

    pub fn values(&self, profile: &'a JavaProfile) -> JavaObjectArrayIterator {
        JavaObjectArrayIterator::new(profile, &self.array)
    }

    pub fn class_id(&self) -> ClassId {
        ClassId::from(self.array.array_class_obj_id())
    }

    pub fn class_name(&self, profile: &'a JavaProfile) -> Option<&str> {
        profile
            .get_class_by_id(&self.class_id())
            .map(|c| c.name(profile))
    }
}
