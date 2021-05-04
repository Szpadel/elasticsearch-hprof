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
