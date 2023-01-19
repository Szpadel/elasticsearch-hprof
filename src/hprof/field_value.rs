use std::borrow::Cow;

use jvm_hprof::heap_dump::FieldValue;

use super::{JavaInstance, JavaObjectArray, JavaPrimitiveArray, JavaProfile, Object};

pub enum JavaLocalValue<'a> {
    Object(&'a JavaInstance<'a>),
    ObjectArray(&'a JavaObjectArray<'a>),
    PrimitiveArray(&'a JavaPrimitiveArray<'a>),
    Boolean(bool),
    Char(u16),
    Float(f32),
    Double(f64),
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Null,
}

impl<'a> JavaLocalValue<'a> {
    pub fn type_name(&'a self, profile: &'a JavaProfile) -> Cow<str> {
        match self {
            JavaLocalValue::Object(o) => o
                .class(profile)
                .map(|c| c.name(profile))
                .unwrap_or("Object")
                .into(),
            JavaLocalValue::ObjectArray(oa) => {
                format!("{}[]", oa.class_name(profile).unwrap_or("unknown")).into()
            }
            JavaLocalValue::PrimitiveArray(a) => format!("{}[]", a.value_type()).into(),
            JavaLocalValue::Boolean(b) => {
                format!("bool({})", if *b { "true" } else { "false" }).into()
            }
            JavaLocalValue::Char(ch) => format!("char({ch})").into(),
            JavaLocalValue::Float(f) => format!("float({f})").into(),
            JavaLocalValue::Double(d) => format!("double({d})").into(),
            JavaLocalValue::Byte(b) => format!("byte({b})").into(),
            JavaLocalValue::Short(s) => format!("short({s})").into(),
            JavaLocalValue::Int(i) => format!("int({i})").into(),
            JavaLocalValue::Long(l) => format!("l({l})").into(),
            JavaLocalValue::Null => "null".into(),
        }
    }
}

#[derive(Debug)]
pub struct JavaFieldValue<'a> {
    name: &'a str,
    field: FieldValue,
}

impl<'a> JavaFieldValue<'a> {
    pub fn new(name: &'a str, field: FieldValue) -> Self {
        Self { name, field }
    }

    pub fn name(&self) -> &'a str {
        self.name
    }

    pub fn value(&self, profile: &'a JavaProfile) -> JavaLocalValue {
        match self.field {
            FieldValue::ObjectId(id) => id
                .and_then(|id| {
                    profile.objects.get(&id.into()).map(|obj| match obj {
                        Object::Instance(obj) => JavaLocalValue::Object(obj),
                        Object::Array(arr) => JavaLocalValue::ObjectArray(arr),
                        Object::PrimitiveArray(arr) => JavaLocalValue::PrimitiveArray(arr),
                    })
                })
                .unwrap_or(JavaLocalValue::Null),
            FieldValue::Boolean(bool) => JavaLocalValue::Boolean(bool),
            FieldValue::Char(ch) => JavaLocalValue::Char(ch),
            FieldValue::Float(f) => JavaLocalValue::Float(f),
            FieldValue::Double(d) => JavaLocalValue::Double(d),
            FieldValue::Byte(b) => JavaLocalValue::Byte(b),
            FieldValue::Short(s) => JavaLocalValue::Short(s),
            FieldValue::Int(i) => JavaLocalValue::Int(i),
            FieldValue::Long(l) => JavaLocalValue::Long(l),
        }
    }
}
