use arrow::array::{Array, ArrayEqual, ArrayRef, PrimitiveArray, UInt32Array};
use datafusion::derive_df::{compose_udf, derive_udf};
use std::sync::Arc;

#[derive_udf]
fn plus(a: u32, b: u32) -> u32 {
    a + b
}

fn mul(a: u32, b: u32) -> u32 {
    a * b
}

fn main() {
    let a: UInt32Array = vec![1, 2, 3].into();
    let b: UInt32Array = vec![1, 2, 3].into();

    let a: ArrayRef = Arc::new(a);
    let b: ArrayRef = Arc::new(b);

    let res = plus_udf(&[a.clone(), b.clone()]);
    let arr = res.as_any().downcast_ref::<UInt32Array>().unwrap();
    // assert_eq!(arr, [2, 4, 6]);

    let comp_fn = compose_udf!(
        fn mul(a: u32, b: u32) -> u32 {
            a * b
        }
    );

    let res = comp_fn(&[a, b]);
    array_eq::<UInt32Array, u32>(res, vec![1u32, 4, 9]);
}

fn array_eq<K, R>(lhs: ArrayRef, rhs: Vec<R>)
where
    K: From<Vec<R>>,
    K: Array,
{
    let rhs: K = rhs.into();

    assert!(lhs.equals(&rhs));
}
