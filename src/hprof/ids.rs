use std::fmt::Display;

use jvm_hprof::Id;

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct StringId(Id);

impl From<Id> for StringId {
    fn from(val: Id) -> Self {
        StringId(val)
    }
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct ClassId(Id);
impl From<Id> for ClassId {
    fn from(val: Id) -> Self {
        ClassId(val)
    }
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct ObjectId(Id);
impl From<Id> for ObjectId {
    fn from(val: Id) -> Self {
        ObjectId(val)
    }
}

impl Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:#08X}", &self.0))
    }
}
