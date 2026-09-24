//! Whole-number arguments that refuse what N-API's own conversions let through.
//!
//! N-API reads an integer argument with `napi_get_value_uint32` or `napi_get_value_int32`,
//! which keep the bottom 32 bits of a number outside the range and drop a fraction: `-1`
//! reaches a `u32` as 4294967295, 4294967296 reaches it as 0, and `2.5` reaches a `u8` as 2.
//! Every integer the binding takes from JavaScript, as an argument or as a field of an options
//! object, is one of the types here instead. Each reads the value as a double and refuses
//! anything that is not a whole number inside its range, naming what it was given.
//!
//! The types carry the names of the primitives they check, so the generated TypeScript still
//! declares `number` for each of them. A count that can pass 2^32, such as a clock in
//! microseconds, is taken as a plain number and read with [`whole`] instead, since a `u64`
//! argument reaches TypeScript as a `bigint`.

#![allow(non_camel_case_types)]

use napi::bindgen_prelude::{FromNapiValue, ToNapiValue, TypeName, ValidateNapiValue};
use napi::{sys, Error, Status, ValueType};

/// A whole number that JavaScript passed in, already checked against the range of `T`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Whole<T>(T);

impl<T> Whole<T> {
    /// Unwraps the checked value.
    ///
    /// # Returns
    ///
    /// The number, inside the range of `T`.
    pub fn get(self) -> T {
        self.0
    }
}

impl<T> From<T> for Whole<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Whole<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// An integer type a JavaScript number is checked against.
pub trait Bounded: Copy + ToNapiValue {
    /// The name the type goes by in Rust.
    const NAME: &'static str;
    /// The smallest value the type holds, as a JavaScript number.
    const LOW: f64;
    /// The largest value the type holds, as a JavaScript number.
    const HIGH: f64;

    /// Narrows a whole number already known to lie between `LOW` and `HIGH`.
    ///
    /// # Arguments
    ///
    /// * `value` - the whole number to narrow.
    ///
    /// # Returns
    ///
    /// The same number as this type.
    fn narrow(value: f64) -> Self;
}

macro_rules! bounded {
    ($($t:ident: $low:expr, $high:expr;)*) => {
        $(
            impl Bounded for core::primitive::$t {
                const NAME: &'static str = stringify!($t);
                const LOW: f64 = $low;
                const HIGH: f64 = $high;

                fn narrow(value: f64) -> Self {
                    value as core::primitive::$t
                }
            }

            #[doc = concat!("A `", stringify!($t), "` checked on the way in from JavaScript.")]
            pub type $t = Whole<core::primitive::$t>;
        )*
    };
}

bounded! {
    u8: 0.0, 255.0;
    u16: 0.0, 65_535.0;
    u32: 0.0, 4_294_967_295.0;
    i8: -128.0, 127.0;
    i16: -32_768.0, 32_767.0;
    i32: -2_147_483_648.0, 2_147_483_647.0;
    i64: -9_007_199_254_740_991.0, 9_007_199_254_740_991.0;
}

impl Bounded for core::primitive::u64 {
    const NAME: &'static str = "u64";
    const LOW: f64 = 0.0;
    const HIGH: f64 = 9_007_199_254_740_991.0;

    fn narrow(value: f64) -> Self {
        value as core::primitive::u64
    }
}

/// Checks a number JavaScript passed in against the range of `T`.
///
/// # Arguments
///
/// * `value` - the number as JavaScript holds it.
///
/// # Returns
///
/// The number as `T`.
///
/// # Errors
///
/// When `value` is not finite, has a fraction, or lies outside the range of `T`.
pub fn whole<T: Bounded>(value: f64) -> napi::Result<T> {
    if value.fract() == 0.0 && (T::LOW..=T::HIGH).contains(&value) {
        return Ok(T::narrow(value));
    }
    let shown = if value.is_nan() {
        "NaN".to_owned()
    } else if value.is_infinite() {
        if value > 0.0 { "Infinity" } else { "-Infinity" }.to_owned()
    } else {
        value.to_string()
    };
    Err(refusal::<T>(&shown))
}

fn refusal<T: Bounded>(given: &str) -> Error {
    Error::new(
        Status::InvalidArg,
        format!(
            "a value must be a whole number from {} to {}, not {given}",
            T::LOW,
            T::HIGH
        ),
    )
}

fn kind(value_type: sys::napi_valuetype) -> &'static str {
    match value_type {
        sys::ValueType::napi_undefined => "undefined",
        sys::ValueType::napi_null => "null",
        sys::ValueType::napi_boolean => "a boolean",
        sys::ValueType::napi_string => "a string",
        sys::ValueType::napi_symbol => "a symbol",
        sys::ValueType::napi_function => "a function",
        sys::ValueType::napi_bigint => "a bigint",
        _ => "an object",
    }
}

impl<T: Bounded> TypeName for Whole<T> {
    fn type_name() -> &'static str {
        T::NAME
    }

    fn value_type() -> ValueType {
        ValueType::Number
    }
}

impl<T: Bounded> ValidateNapiValue for Whole<T> {}

impl<T: Bounded> FromNapiValue for Whole<T> {
    unsafe fn from_napi_value(env: sys::napi_env, napi_val: sys::napi_value) -> napi::Result<Self> {
        let mut value_type = 0;
        if unsafe { sys::napi_typeof(env, napi_val, &mut value_type) } != sys::Status::napi_ok {
            return Err(refusal::<T>("a value N-API could not read"));
        }
        if value_type != sys::ValueType::napi_number {
            return Err(refusal::<T>(kind(value_type)));
        }
        let mut value = 0.0;
        if unsafe { sys::napi_get_value_double(env, napi_val, &mut value) } != sys::Status::napi_ok
        {
            return Err(refusal::<T>("a value N-API could not read"));
        }
        whole(value).map(Whole)
    }
}

impl<T: Bounded> ToNapiValue for Whole<T> {
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        unsafe { T::to_napi_value(env, val.0) }
    }
}

/// Unwraps a checked number that JavaScript may have left out.
pub trait OptionalWhole<T> {
    /// Unwraps the checked value, if there is one.
    ///
    /// # Returns
    ///
    /// The number, or `None` when JavaScript passed none.
    fn get(self) -> Option<T>;
}

impl<T> OptionalWhole<T> for Option<Whole<T>> {
    fn get(self) -> Option<T> {
        self.map(Whole::get)
    }
}

/// Unwraps every checked number in a list.
///
/// # Arguments
///
/// * `values` - the checked numbers.
///
/// # Returns
///
/// The numbers, in the same order.
pub fn all<T>(values: Vec<Whole<T>>) -> Vec<T> {
    values.into_iter().map(Whole::get).collect()
}
