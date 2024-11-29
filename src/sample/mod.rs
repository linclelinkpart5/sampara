mod conv;

pub use self::conv::{FromSample, IntoSample};

use core::fmt::Debug;

use num_traits::{Float, FloatConst, Signed};

/// A trait for working generically across different sample format types, both
/// in terms of representation (integral versus floating-point) and bitsize.
pub trait Sample: Copy + Clone + PartialOrd + PartialEq + Debug {
    /// The equilibrium value for the wave that this sample type represents.
    /// This is normally the value that is equal distance from both the min and
    /// max ranges of the sample, i.e. the "zero amplitude" value.
    const EQUILIBRIUM: Self;

    /// When adding two [`Sample`]s together, it is necessary to convert
    /// both temporarily into some mutual signed format. This associated type
    /// represents the [`Sample`] type to convert to for optimal/lossless
    /// addition.
    type Signed: SignedSample; // + Duplex<Self>;

    /// When multiplying two [`Sample`]s together, it is necessary to convert
    /// both temporarily into some mutual float format. This associated type
    /// represents the [`Sample`] type to convert to for optimal/lossless
    /// multiplication.
    type Float: FloatSample; // + Duplex<Self>;
}

/// A macro used to simplify the implementation of [`Sample`].
macro_rules! impl_sample {
    ($($T:ty: {
       Signed: $Signed:ty,
       Float: $Float:ty,
       EQUILIBRIUM: $EQUILIBRIUM:expr }),* $(,)?) =>
    {
        $(
            impl Sample for $T {
                type Signed = $Signed;
                type Float = $Float;
                const EQUILIBRIUM: Self = $EQUILIBRIUM;
            }
        )*
    }
}

// Implements [`Sample`] for all of the following primitive types.
impl_sample! {
    i8:   { Signed: i8,   Float: f32, EQUILIBRIUM: 0 },
    i16:  { Signed: i16,  Float: f32, EQUILIBRIUM: 0 },
    i32:  { Signed: i32,  Float: f32, EQUILIBRIUM: 0 },
    i64:  { Signed: i64,  Float: f64, EQUILIBRIUM: 0 },
    i128: { Signed: i128, Float: f64, EQUILIBRIUM: 0 },
    u8:   { Signed: i8,   Float: f32, EQUILIBRIUM: 1 << 7 },
    u16:  { Signed: i16,  Float: f32, EQUILIBRIUM: 1 << 15 },
    u32:  { Signed: i32,  Float: f32, EQUILIBRIUM: 1 << 31 },
    u64:  { Signed: i64,  Float: f64, EQUILIBRIUM: 1 << 63 },
    u128: { Signed: i128, Float: f64, EQUILIBRIUM: 1 << 127 },
    f32:  { Signed: f32,  Float: f32, EQUILIBRIUM: 0.0 },
    f64:  { Signed: f64,  Float: f64, EQUILIBRIUM: 0.0 },
}

/// Integral and floating-point [`Sample`] types whose equilibrium is at 0.
///
/// [`Sample`]s often need to be converted to some mutual [`SignedSample`] type
/// for addition.
pub trait SignedSample: Sample<Signed = Self> + Signed {}

macro_rules! impl_signed_sample { ($($T:ty)*) => { $( impl SignedSample for $T {} )* } }
impl_signed_sample!(i8 i16 i32 i64 i128 f32 f64);

/// Floating-point [`Sample`] types, represented as values in the interval
/// [-1.0, 1.0).
///
/// [`Sample`]s often need to be converted to some mutual [`FloatSample`] type
/// for scaling.
pub trait FloatSample:
    Sample<Signed = Self, Float = Self> + SignedSample /*+ Duplex<f32> + Duplex<f64>*/ + Float + FloatConst
{
}

impl FloatSample for f32 {}

impl FloatSample for f64 {}
