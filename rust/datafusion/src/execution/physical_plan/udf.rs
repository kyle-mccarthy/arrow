use crate::error::{ExecutionError, Result};
use crate::logicalplan::ScalarValue;
use std::convert::TryInto;

macro_rules! next_arg {
    ($ARGS:ident, $TYPE:ty) => {{
        let arg: ScalarValue = $ARGS.next().ok_or_else(|| {
            ExecutionError::General("Expected additional arg found None".to_string())
        })?;

        let arg: Option<$TYPE> = arg.try_into()?;

        arg.ok_or_else(|| ExecutionError::General("Expected non-null value".to_string()))
    }};
}

/// TODO :: ScalarFunction definition
pub type ScalarFunction = Box<dyn Fn(Vec<ScalarValue>) -> Result<ScalarValue>>;

macro_rules! impl_compose {
    ($FN:ident, $($ARG:ident),*) => {
        /// Bind UDF with parameters
        pub fn $FN<$($ARG,)* R: Into<ScalarValue>, F: 'static + Fn($($ARG,)*) -> R>(f: F) -> ScalarFunction where
            $(ScalarValue: TryInto<Option<$ARG>, Error = ExecutionError>,)*
        {
            Box::new(move |args: Vec<ScalarValue>| {
                let mut args: std::vec::IntoIter<ScalarValue> = args.into_iter();

                Ok(
                    f(
                        $(
                            next_arg!(args, $ARG)?,
                        )*
                    ).into()
                )
            })
        }

    }
}

impl_compose!(compose1, A);
impl_compose!(compose2, A, B);

// /// Bind UDF with single parameter
// pub fn compose1<T, R: Into<ScalarValue>, F: 'static + Fn(T) -> R>(f: F) -> ScalarFunction
// where
//     ScalarValue: TryInto<Option<T>, Error = ExecutionError>,
// {
//     Box::new(move |args: Vec<ScalarValue>| {
//         let mut args = args.into_iter();

//         let arg1: T = next_arg!(args)?;

//         Ok(f(arg1).into())
//     })
// }

#[cfg(test)]
mod udf_tests {
    use super::*;

    fn adds_10(x: u32) -> u32 {
        x + 10u32
    }

    #[test]
    fn it() {
        let f = compose1(adds_10);
        let v = vec![f];

        let res = v[0](vec![ScalarValue::UInt32(20)]);
        assert_eq!(res.unwrap(), ScalarValue::UInt32(30));
    }
}
