use crate::error::{ExecutionError, Result};
use crate::logicalplan::ScalarValue;

use arrow::array::{ArrayDataRef, ArrayRef};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;

use std::collections::BTreeMap;
use std::sync::Arc;

pub struct ScalarArray {
    data: ArrayDataRef,
    value_offsets: *const i32,
    value_data: *const u8,
}

/// Params for the UDF
pub struct Params {
    inner: Vec<Field>,
    map: BTreeMap<String, usize>,
}

impl Params {
    /// Create function params from list of fields
    pub fn new(fields: Vec<Field>) -> Params {
        let mut map: BTreeMap<String, usize> = BTreeMap::new();

        fields.iter().enumerate().for_each(|(k, v)| {
            map.insert(v.name().clone(), k);
        });

        Params { inner: fields, map }
    }
}

impl From<Vec<Field>> for Params {
    fn from(fields: Vec<Field>) -> Self {
        Params::new(fields)
    }
}

/// Context of a functions call
pub struct CallContext {
    data: Vec<ArrayRef>,
    schema: Arc<Schema>,
    iteration: usize,
}

impl CallContext {}

pub trait UserDefinedFunction {
    fn evaluate(&self, ctx: &CallContext);
    fn is_aggregate(&self) -> bool;
}
