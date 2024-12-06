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
    type Signed: SignedSample + FromSample<Self>;

    /// When multiplying two [`Sample`]s together, it is necessary to convert
    /// both temporarily into some mutual float format. This associated type
    /// represents the [`Sample`] type to convert to for optimal/lossless
    /// multiplication.
    type Float: FloatSample + FromSample<Self>;

    /// Adds/offsets the amplitude of this [`Sample`] by a signed amplitude.
    ///
    /// This value will be converted into [`Self::Signed`], then added. The
    /// result will then be converted back into [`Self`]. This double conversion
    /// is to correctly handle the addition of unsigned signal formats.
    ///
    /// ```
    /// use sampara::Sample;
    ///
    /// fn main() {
    ///     assert_eq!(192i16.add_amp(-128), 64);
    ///     assert_eq!(0.25f32.add_amp(0.5), 0.75);
    /// }
    /// ```
    #[inline]
    fn add_amp(self, amp: Self) -> Self
    where
        Self: SignedSample,
    {
        self + amp
    }

    /// Subtracts/offsets the amplitude of this [`Sample`] by a signed amplitude.
    ///
    /// This value will be converted into [`Self::Signed`], then subtracted. The
    /// result will then be converted back into [`Self`]. This double conversion
    /// is to correctly handle the subtraction of unsigned signal formats.
    ///
    /// ```
    /// use sampara::Sample;
    ///
    /// fn main() {
    ///     assert_eq!(192i16.sub_amp(-128), 320);
    ///     assert_eq!(0.25f32.sub_amp(0.5), -0.25);
    /// }
    /// ```
    #[inline]
    fn sub_amp(self, amp: Self) -> Self
    where
        Self: SignedSample,
    {
        self - amp
    }

    /// Multiplies/scales the amplitude of this [`Sample`] by a float amplitude.
    ///
    /// ```
    /// use sampara::Sample;
    ///
    /// fn main() {
    ///     assert_eq!(0.4f32.mul_amp(0.5), 0.2);
    ///     assert_eq!(0.5f64.mul_amp(-2.0), -1.0);
    ///     assert_eq!(0.5f32.mul_amp(0.0), 0.0);
    ///     assert_eq!(0.5f32.mul_amp(1.0), 0.5);
    /// }
    /// ```
    #[inline]
    fn mul_amp(self, amp: Self) -> Self
    where
        Self: FloatSample,
    {
        self * amp
    }

    /// Divides/scales the amplitude of this [`Sample`] by a float amplitude.
    ///
    /// ```
    /// use sampara::Sample;
    ///
    /// fn main() {
    ///     assert_eq!(0.4f32.div_amp(0.5), 0.8);
    ///     assert_eq!(0.5f64.div_amp(-2.0), -0.25);
    ///     assert!(0.5f32.div_amp(0.0).is_infinite());
    ///     assert_eq!(0.5f32.div_amp(1.0), 0.5);
    /// }
    /// ```
    #[inline]
    fn div_amp(self, amp: Self) -> Self
    where
        Self: FloatSample,
    {
        self / amp
    }
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
    u8:   { Signed: i8,   Float: f32, EQUILIBRIUM: 1u8.reverse_bits() },
    u16:  { Signed: i16,  Float: f32, EQUILIBRIUM: 1u16.reverse_bits() },
    u32:  { Signed: i32,  Float: f32, EQUILIBRIUM: 1u32.reverse_bits() },
    u64:  { Signed: i64,  Float: f64, EQUILIBRIUM: 1u64.reverse_bits() },
    u128: { Signed: i128, Float: f64, EQUILIBRIUM: 1u128.reverse_bits() },
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
