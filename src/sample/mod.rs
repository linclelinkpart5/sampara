pub mod conv;

pub use conv::{FromSample, IntoSample, Duplex};

/// A trait for working generically across different sample format types, both
/// in terms of representation (integer versus float) and bitsize.
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
}

/// A macro used to simplify the implementation of `Sample`.
macro_rules! impl_sample {
    ($($T:ty: {
       Signed: $Addition:ty,
       Float: $Modulation:ty,
       EQUILIBRIUM: $EQUILIBRIUM:expr }),* $(,)?) =>
    {
        $(
            impl Sample for $T {
                // type Signed = $Addition;
                // type Float = $Modulation;
                const EQUILIBRIUM: Self = $EQUILIBRIUM;
            }
        )*
    }
}

// Implements `Sample` for all of the following primitive types.
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
