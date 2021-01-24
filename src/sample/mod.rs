pub mod conv;

pub use conv::{FromSample, IntoSample, Duplex};

use core::ops::{Add, Sub, Mul, Div, Neg};

/// A trait for working generically across different sample format types, both
/// in terms of representation (integral versus floating-point) and bitsize.
pub trait Sample: Copy + Clone + PartialOrd + PartialEq {
    /// The equilibrium value for the wave that this sample type represents.
    /// This is normally the value that is equal distance from both the min and
    /// max ranges of the sample, i.e. the "zero amplitude" value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sampara::Sample;
    ///
    /// fn main() {
    ///     assert_eq!(0.0, f32::EQUILIBRIUM);
    ///     assert_eq!(0, i32::EQUILIBRIUM);
    ///     assert_eq!(128, u8::EQUILIBRIUM);
    ///     assert_eq!(32_768_u16, Sample::EQUILIBRIUM);
    /// }
    /// ```
    const EQUILIBRIUM: Self;

    /// When adding two [`Sample`]s together, it is necessary to convert
    /// both temporarily into some mutual signed format. This associated type
    /// represents the [`Sample`] type to convert to for optimal/lossless
    /// addition.
    type Signed: SignedSample + Duplex<Self>;

    /// When multiplying two [`Sample`]s together, it is necessary to convert
    /// both temporarily into some mutual float format. This associated type
    /// represents the [`Sample`] type to convert to for optimal/lossless
    /// multiplication.
    type Float: FloatSample + Duplex<Self>;
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
    i8:  { Signed: i8,  Float: f32, EQUILIBRIUM: 0 },
    i16: { Signed: i16, Float: f32, EQUILIBRIUM: 0 },
    i32: { Signed: i32, Float: f32, EQUILIBRIUM: 0 },
    i64: { Signed: i64, Float: f64, EQUILIBRIUM: 0 },
    u8:  { Signed: i8,  Float: f32, EQUILIBRIUM: 128 },
    u16: { Signed: i16, Float: f32, EQUILIBRIUM: 32_768 },
    u32: { Signed: i32, Float: f32, EQUILIBRIUM: 2_147_483_648 },
    u64: { Signed: i64, Float: f64, EQUILIBRIUM: 9_223_372_036_854_775_808 },
    f32: { Signed: f32, Float: f32, EQUILIBRIUM: 0.0 },
    f64: { Signed: f64, Float: f64, EQUILIBRIUM: 0.0 },
}

/// Integral and floating-point [`Sample`] types whose equilibrium is at 0.
///
/// [`Sample`]s often need to be converted to some mutual [`SignedSample`] type
/// for addition.
pub trait SignedSample:
    Sample<Signed = Self>
    + Add<Output = Self>
    + Sub<Output = Self>
    + Neg<Output = Self>
{
}
macro_rules! impl_signed_sample { ($($T:ty)*) => { $( impl SignedSample for $T {} )* } }
impl_signed_sample!(i8 i16 i32 i64 f32 f64);

/// Floating-point [`Sample`] types, represented as values in the interval
/// [-1.0, 1.0).
///
/// [`Sample`]s often need to be converted to some mutual [`FloatSample`] type
/// for scaling and modulation.
pub trait FloatSample:
    Sample<Signed = Self, Float = Self>
    + SignedSample
    + Mul<Output = Self>
    + Div<Output = Self>
    + Duplex<f32>
    + Duplex<f64>
{
    /// Represents the multiplicative identity of the floating point signal.
    const IDENTITY: Self;

    /// Calculate the square root of the sample.
    fn sample_sqrt(self) -> Self;
}

impl FloatSample for f32 {
    const IDENTITY: Self = 1.0;
    #[inline]
    fn sample_sqrt(self) -> Self {
        self.sqrt()
    }
}

impl FloatSample for f64 {
    const IDENTITY: Self = 1.0;
    #[inline]
    fn sample_sqrt(self) -> Self {
        self.sqrt()
    }
}
