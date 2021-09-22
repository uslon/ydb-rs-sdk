use crate::errors::{Error, Result};
use ydb_protobuf::generated::ydb;

/// Represent value, send or received from ydb
/// That enum will be grow, when add support of new types
#[derive(Debug, PartialEq)]
pub enum YdbValue {
    NULL,
    BOOL(bool),
    INT32(i32),
    UINT32(u32),
    INT64(i64),
    UINT64(u64),
    FLOAT32(f32),
    FLOAT64(f64),
    BYTES(Vec<u8>),
    TEXT(String),
}

impl YdbValue {
    pub(crate) fn from_proto(proto_value: ydb::Value) -> Result<Self> {
        use ydb_protobuf::generated::ydb::value::Value::*;
        println!("from proto item: {:?}", proto_value);
        let val = match proto_value.value {
            None => return Err(Error::from("null value in proto value item")),
            Some(val) => match val {
                BoolValue(val) => YdbValue::BOOL(val),
                Int32Value(val) => YdbValue::INT32(val),
                Uint32Value(val) => YdbValue::UINT32(val),
                Int64Value(val) => YdbValue::INT64(val),
                Uint64Value(val) => YdbValue::UINT64(val),
                FloatValue(val) => YdbValue::FLOAT32(val),
                DoubleValue(val) => YdbValue::FLOAT64(val),
                BytesValue(val) => YdbValue::BYTES(val),
                TextValue(val) => YdbValue::TEXT(val),
                NullFlagValue(_) => YdbValue::NULL,
                NestedValue(_) => return Err(Error::from("not implemented read nested")),
                Low128(_) => return Err(Error::from("not implemented read i128")),
            },
        };
        return Ok(val);
    }

    pub(crate) fn to_typed_value(self) -> ydb::TypedValue {
        use ydb::r#type::PrimitiveTypeId as pt;
        use ydb::value::Value as pv;

        fn to_typed(t: pt, v: pv) -> ydb::TypedValue {
            ydb::TypedValue {
                r#type: Some(ydb::Type {
                    r#type: Some(ydb::r#type::Type::TypeId(t.into())),
                }),
                value: Some(ydb::Value {
                    value: Some(v),
                    ..ydb::Value::default()
                }),
            }
        }

        match self {
            Self::NULL => panic!("unimplemented"),
            Self::INT32(val) => to_typed(pt::Int32, pv::Int32Value(val)),
            Self::BOOL(val) => to_typed(pt::Bool, pv::BoolValue(val)),
            Self::UINT32(val) => to_typed(pt::Uint32, pv::Uint32Value(val)),
            Self::INT64(val) => to_typed(pt::Int64, pv::Int64Value(val)),
            Self::UINT64(val) => to_typed(pt::Uint64, pv::Uint64Value(val)),
            Self::FLOAT32(val) => to_typed(pt::Float, pv::FloatValue(val)),
            Self::FLOAT64(val) => to_typed(pt::Double, pv::DoubleValue(val)),
            Self::BYTES(val) => to_typed(pt::String, pv::BytesValue(val)),
            Self::TEXT(val) => to_typed(pt::Utf8, pv::TextValue(val)),
        }
    }
}

#[derive(Debug)]
pub struct Column {
    pub name: String,
}