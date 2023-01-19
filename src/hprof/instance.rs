use std::iter::FromIterator;

use ahash::AHashMap;
use jvm_hprof::heap_dump::{FieldDescriptors, Instance};

use super::{
    ClassId, JavaClass, JavaFieldValue, JavaLocalValue, JavaObjectArray, JavaPrimitiveArray,
    JavaProfile, ObjectId,
};

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

    pub fn name(&'a self, profile: &'a JavaProfile) -> Option<&'a str> {
        self.class(profile).map(|class| class.name(profile))
    }

    pub fn find_field_by_name(
        &'a self,
        profile: &'a JavaProfile,
        name: &str,
    ) -> Option<JavaFieldValue> {
        self.local_fields(profile).find(|f| f.name() == name)
    }

    pub fn class(&'a self, profile: &'a JavaProfile) -> Option<&'a JavaClass> {
        let class_id = ClassId::from(self.instance.class_obj_id());
        profile.get_class_by_id(&class_id)
    }

    pub fn fields(&'a self, profile: &'a JavaProfile) -> JavaInstanceFields<'a> {
        JavaInstanceFields::new(self.local_fields(profile))
    }
}

pub struct JavaInstanceFields<'a> {
    pub fields: AHashMap<&'a str, JavaFieldValue<'a>>,
}

macro_rules! impl_java_value_output {
    ( $( $struct_name:ty )+, $( $enum:tt )+ ) => {$(
        impl<'a> FieldValueOutput<'a> for $struct_name {
            fn extract_value(value: &JavaLocalValue<'a>) -> Option<Self> {
                if let &JavaLocalValue::$enum(r) = value {
                    Some(r)
                }else {
                    None
                }
            }
        }
    )*};
}

pub trait FieldValueOutput<'a>
where
    Self: Sized,
{
    fn extract_value(value: &JavaLocalValue<'a>) -> Option<Self>;
}
impl_java_value_output!(&'a JavaInstance<'a>, Object);
impl_java_value_output!(&'a JavaObjectArray<'a>, ObjectArray);
impl_java_value_output!(&'a JavaPrimitiveArray<'a>, PrimitiveArray);
impl_java_value_output!(bool, Boolean);
impl_java_value_output!(u16, Char);
impl_java_value_output!(f32, Float);
impl_java_value_output!(f64, Double);
impl_java_value_output!(i8, Byte);
impl_java_value_output!(i16, Short);
impl_java_value_output!(i32, Int);
impl_java_value_output!(i64, Long);

impl<'a> JavaInstanceFields<'a> {
    fn new(fields_iter: LocalFieldsIterator<'a>) -> Self {
        Self {
            fields: fields_iter.collect(),
        }
    }

    pub fn value<T>(&'a self, profile: &'a JavaProfile, name: &str) -> Option<T>
    where
        T: FieldValueOutput<'a>,
    {
        self.fields
            .get(name)
            .and_then(|f| T::extract_value(&f.value(profile)))
    }
}
