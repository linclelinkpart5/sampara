pub mod conv;

pub use conv::{ConvertFrom, ConvertInto, Duplex};

use num_traits::{Float, FloatConst, Signed};

/// A trait for working generically across different sample format types, both
/// in terms of representation (integral versus floating-point) and bitsize.
pub trait Sample: Copy + Clone + PartialOrd + PartialEq {
    /// The equilibrium value for the wave that this sample type represents.
    /// This is normally the value that is equal distance from both the min and
    /// max ranges of the sample, i.e. the "zero amplitude" value.
    ///
    /// ```
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

    /// Converts this [`Sample`] into another [`Sample`] type.
    ///
    /// ```
    /// use sampara::Sample;
    ///
    /// fn main() {
    ///     assert_eq!(0.0.into_sample::<i32>(), 0);
    ///     assert_eq!(0.0.into_sample::<u8>(), 128);
    ///     assert_eq!((-1.0).into_sample::<u8>(), 0);
    /// }
    /// ```
    #[inline]
    fn into_sample<S>(self) -> S
    where
        Self: ConvertInto<S>,
        S: Sample,
    {
        self.convert_into()
    }

    /// Creates an instance of this [`Sample`] from another [`Sample`] type.
    ///
    /// ```
    /// use sampara::Sample;
    ///
    /// fn main() {
    ///     assert_eq!(f32::from_sample(128u8), 0.0);
    ///     assert_eq!(i8::from_sample(-1.0), -128);
    ///     assert_eq!(u16::from_sample(0.5), 49152);
    /// }
    /// ```
    #[inline]
    fn from_sample<S>(s: S) -> Self
    where
        Self: ConvertFrom<S>,
        S: Sample,
    {
        ConvertFrom::convert_from(s)
    }

    /// Converts this [`Sample`] into its corresponding [`Self::Signed`] type.
    ///
    /// This is a simple wrapper around [`Sample::into_sample`] to provide
    /// extra type inference convenience in some cases.
    ///
    /// ```
    /// use sampara::Sample;
    ///
    /// fn main() {
    ///     assert_eq!(128_u8.into_signed_sample(), 0_i8);
    ///     assert_eq!(128_u16.into_signed_sample(), -32640_i16);
    ///     assert_eq!((-128_i8).into_signed_sample(), -128_i8);
    /// }
    /// ```
    fn into_signed_sample(self) -> Self::Signed {
        self.into_sample()
    }

    /// Converts this [`Sample`] into its corresponding [`Self::Float`] type.
    ///
    /// This is a simple wrapper around [`Sample::into_sample`] to provide
    /// extra type inference convenience in some cases.
    ///
    /// ```
    /// use sampara::Sample;
    ///
    /// fn main() {
    ///     assert_eq!(128_u8.into_float_sample(), 0.0_f32);
    ///     assert_eq!(128_u16.into_float_sample(), -0.99609375_f32);
    ///     assert_eq!((-128_i8).into_float_sample(), -1.0_f32);
    /// }
    /// ```
    fn into_float_sample(self) -> Self::Float {
        self.into_sample()
    }

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
    ///     assert_eq!(0.25.add_amp(0.5), 0.75);
    ///     assert_eq!(192u8.add_amp(-128), 64);
    /// }
    /// ```
    #[inline]
    fn add_amp(self, amp: Self::Signed) -> Self {
        let self_s = self.into_signed_sample();
        (self_s + amp).into_sample()
    }

    /// Multiplies/scales the amplitude of this [`Sample`] by a float amplitude.
    ///
    /// This value will be converted into [`Self::Float`], then multiplied. The
    /// result will then be converted back into [`Self`]. This double conversion
    /// is to correctly handle the multiplication of integer signal formats.
    ///
    /// ```
    /// use sampara::Sample;
    ///
    /// fn main() {
    ///     assert_eq!(64_i8.mul_amp(0.5), 32);
    ///     assert_eq!(0.5.mul_amp(-2.0), -1.0);
    ///     assert_eq!(64_u8.mul_amp(0.0), 128);
    /// }
    /// ```
    #[inline]
    fn mul_amp(self, amp: Self::Float) -> Self {
        let self_f = self.into_float_sample();
        (self_f * amp).into_sample()
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
    + Signed
{}

macro_rules! impl_signed_sample { ($($T:ty)*) => { $( impl SignedSample for $T {} )* } }
impl_signed_sample!(i8 i16 i32 i64 f32 f64);

pub trait Sqrt {
    /// Square root.
    ///
    /// ```
    /// use sampara::sample::Sqrt;
    ///
    /// fn main() {
    ///     assert_eq!(4.0_f32.sqrt(), 2.0);
    ///     assert_eq!(2.0_f64.sqrt(), 1.4142135623730951);
    ///     assert!((-1.0_f64).sqrt().is_nan());
    /// }
    /// ```
    fn sqrt(self) -> Self;
}

impl Sqrt for f32 {
    #[inline(always)]
    fn sqrt(self) -> f32 { self.sqrt() }
}

impl Sqrt for f64 {
    #[inline(always)]
    fn sqrt(self) -> f64 { self.sqrt() }
}

/// Floating-point [`Sample`] types, represented as values in the interval
/// [-1.0, 1.0).
///
/// [`Sample`]s often need to be converted to some mutual [`FloatSample`] type
/// for scaling.
pub trait FloatSample:
    Sample<Signed = Self, Float = Self>
    + SignedSample
    + Duplex<f32>
    + Duplex<f64>
    + Float
    + FloatConst
{}

impl FloatSample for f32 {}

impl FloatSample for f64 {}
