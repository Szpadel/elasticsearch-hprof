use jvm_hprof::heap_dump::{PrimitiveArray, PrimitiveArrayType};

use super::ObjectId;

pub enum PrimitiveArrayValues {
    Boolean(Vec<bool>),
    Char(Vec<u16>),
    Float(Vec<f32>),
    Double(Vec<f64>),
    Byte(Vec<i8>),
    Short(Vec<i16>),
    Int(Vec<i32>),
    Long(Vec<i64>),
}

pub struct JavaPrimitiveArray<'a> {
    array: PrimitiveArray<'a>,
}

impl<'a> JavaPrimitiveArray<'a> {
    pub fn new(array: PrimitiveArray<'a>) -> Self {
        Self { array }
    }

    pub fn id(&self) -> ObjectId {
        self.array.obj_id().into()
    }

    pub fn value_type(&self) -> &str {
        self.array.primitive_type().java_type_name()
    }

    pub fn values(&self) -> PrimitiveArrayValues {
        match self.array.primitive_type() {
            PrimitiveArrayType::Boolean => PrimitiveArrayValues::Boolean(
                self.array
                    .booleans()
                    .unwrap()
                    .filter_map(|i| i.ok())
                    .collect::<Vec<_>>(),
            ),
            PrimitiveArrayType::Char => PrimitiveArrayValues::Char(
                self.array
                    .chars()
                    .unwrap()
                    .filter_map(|i| i.ok())
                    .collect::<Vec<_>>(),
            ),
            PrimitiveArrayType::Float => PrimitiveArrayValues::Float(
                self.array
                    .floats()
                    .unwrap()
                    .filter_map(|i| i.ok())
                    .collect::<Vec<_>>(),
            ),
            PrimitiveArrayType::Double => PrimitiveArrayValues::Double(
                self.array
                    .doubles()
                    .unwrap()
                    .filter_map(|i| i.ok())
                    .collect::<Vec<_>>(),
            ),
            PrimitiveArrayType::Byte => PrimitiveArrayValues::Byte(
                self.array
                    .bytes()
                    .unwrap()
                    .filter_map(|i| i.ok())
                    .collect::<Vec<_>>(),
            ),
            PrimitiveArrayType::Short => PrimitiveArrayValues::Short(
                self.array
                    .shorts()
                    .unwrap()
                    .filter_map(|i| i.ok())
                    .collect::<Vec<_>>(),
            ),
            PrimitiveArrayType::Int => PrimitiveArrayValues::Int(
                self.array
                    .ints()
                    .unwrap()
                    .filter_map(|i| i.ok())
                    .collect::<Vec<_>>(),
            ),
            PrimitiveArrayType::Long => PrimitiveArrayValues::Long(
                self.array
                    .longs()
                    .unwrap()
                    .filter_map(|i| i.ok())
                    .collect::<Vec<_>>(),
            ),
        }
    }
}
