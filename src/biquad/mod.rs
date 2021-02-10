use num_traits::Float;

use crate::{Frame, Duplex, ConvertFrom, ConvertInto};
use crate::sample::FloatSample;

pub trait Param: Float {
    const ZERO: Self;
    const ONE: Self;
    const TWO: Self;
    const HALF: Self;
    const SQRT_2: Self;
    const PI: Self;

    fn a_cap(self) -> Self;
}

impl Param for f32 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;
    const TWO: Self = 2.0;
    const HALF: Self = 0.5;
    const SQRT_2: Self = std::f32::consts::SQRT_2;
    const PI: Self = std::f32::consts::PI;

    fn a_cap(self) -> Self {
        10.0.powf(self / 40.0)
    }
}

impl Param for f64 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;
    const TWO: Self = 2.0;
    const HALF: Self = 0.5;
    const SQRT_2: Self = std::f64::consts::SQRT_2;
    const PI: Self = std::f64::consts::PI;

    fn a_cap(self) -> Self {
        10.0.powf(self / 40.0)
    }
}

pub enum Kind<P>
where
    P: Param,
{
    Allpass,
    Lowpass,
    Highpass,
    Bandpass,
    Notch,
    Peak(P),
    Lowshelf(P),
    Highshelf(P),
}

impl<P> Kind<P>
where
    P: Param,
{
    fn into_params(self, norm_freq: P, q_factor: P) -> Coefficients<P> {
        let omega = P::TWO * P::PI * norm_freq;
        let (omega_s, omega_c) = omega.sin_cos();
        let alpha = omega_s / (P::TWO * q_factor);

        let b0: P;
        let b1: P;
        let b2: P;
        let a0: P;
        let a1: P;
        let a2: P;

        match self {
            Self::Allpass => {
                b0 = P::ONE - alpha;
                b1 = -P::TWO * omega_c;
                b2 = P::ONE + alpha;
                a0 = P::ONE + alpha;
                a1 = -P::TWO * omega_c;
                a2 = P::ONE - alpha;
            },
            Self::Lowpass => {
                b0 = (P::ONE - omega_c) * P::HALF;
                b1 = P::ONE - omega_c;
                b2 = (P::ONE - omega_c) * P::HALF;
                a0 = P::ONE + alpha;
                a1 = -P::TWO * omega_c;
                a2 = P::ONE - alpha;
            },
            Self::Highpass => {
                b0 = (P::ONE + omega_c) * P::HALF;
                b1 = -(P::ONE + omega_c);
                b2 = (P::ONE + omega_c) * P::HALF;
                a0 = P::ONE + alpha;
                a1 = -P::TWO * omega_c;
                a2 = P::ONE - alpha;
            },
            Self::Bandpass => {
                b0 = omega_s * P::HALF;
                b1 = P::ZERO;
                b2 = -(omega_s * P::HALF);
                a0 = P::ONE + alpha;
                a1 = -P::TWO * omega_c;
                a2 = P::ONE - alpha;
            },
            Self::Notch => {
                b0 = P::ONE;
                b1 = -P::TWO * omega_c;
                b2 = P::ONE;
                a0 = P::ONE + alpha;
                a1 = -P::TWO * omega_c;
                a2 = P::ONE - alpha;
            },
            Self::Peak(db_gain) => {
                let a = db_gain.a_cap();

                b0 = P::ONE + alpha * a;
                b1 = -P::TWO * omega_c;
                b2 = P::ONE - alpha * a;
                a0 = P::ONE + alpha / a;
                a1 = -P::TWO * omega_c;
                a2 = P::ONE - alpha / a;
            },
            Self::Lowshelf(db_gain) => {
                let a = db_gain.a_cap();
                let a_p1 = a + P::ONE;
                let a_m1 = a - P::ONE;
                let sqrt_a = a.sqrt();

                b0 = a * (a_p1 - a_m1 * omega_c + P::TWO * alpha * sqrt_a);
                b1 = P::TWO * a * (a_m1 - a_p1 * omega_c);
                b2 = a * (a_p1 - a_m1 * omega_c - P::TWO * alpha * sqrt_a);
                a0 = a_p1 + a_m1 * omega_c + P::TWO * alpha * sqrt_a;
                a1 = -P::TWO * (a_m1 + a_p1 * omega_c);
                a2 = a_p1 + a_m1 * omega_c - P::TWO * alpha * sqrt_a;
            },
            Self::Highshelf(db_gain) => {
                let a = db_gain.a_cap();
                let a_p1 = a + P::ONE;
                let a_m1 = a - P::ONE;
                let sqrt_a = a.sqrt();

                b0 = a * (a_p1 + a_m1 * omega_c + P::TWO * alpha * sqrt_a);
                b1 = -P::TWO * a * (a_m1 + a_p1 * omega_c);
                b2 = a * (a_p1 + a_m1 * omega_c - P::TWO * alpha * sqrt_a);
                a0 = a_p1 - a_m1 * omega_c + P::TWO * alpha * sqrt_a;
                a1 = P::TWO * (a_m1 - a_p1 * omega_c);
                a2 = a_p1 - a_m1 * omega_c - P::TWO * alpha * sqrt_a;
            },
        };

        let norm = a0.recip();

        Coefficients {
            b0: b0 * norm,
            b1: b1 * norm,
            b2: b2 * norm,
            a1: a1 * norm,
            a2: a2 * norm,
        }
    }
}

/// Coefficients for a digital biquad filter.
/// It is assumed that the `a0` coefficient is always normalized to 1.0,
/// and thus not included.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Coefficients<X>
where
    X: Param,
{
    // Transfer function numerator coefficients.
    pub b0: X,
    pub b1: X,
    pub b2: X,

    // Transfer function denominator coefficients.
    pub a1: X,
    pub a2: X,
}

impl<X> Coefficients<X>
where
    X: Param,
{
    pub fn from_kind(kind: Kind<X>, norm_freq: X, q_factor: X) -> Self {
        kind.into_params(norm_freq, q_factor)
    }
}

/// An implementation of a digital biquad filter, using the Direct Form 2
/// Transposed (DF2T) representation.
pub struct Biquad<F, const N: usize>
where
    F: Frame<N>,
    F::Sample: FloatSample + Param,
{
    coeff: Coefficients<F::Sample>,

    // Since biquad filters are second-order, we require two historical buffers.
    // This state is updated each time the filter is applied to a frame.
    t0: F,
    t1: F,
}

impl<F, const N: usize> Biquad<F, N>
where
    F: Frame<N>,
    F::Sample: FloatSample + Param,
{
    pub fn new(coeff: Coefficients<F::Sample>) -> Self {
        Self {
            coeff,
            t0: Frame::EQUILIBRIUM,
            t1: Frame::EQUILIBRIUM,
        }
    }

    /// Performs a single iteration of this filter, calculating a new filtered
    /// `Frame` from an input `Frame`.
    ///
    /// ```rust
    /// use sampara::biquad::{Coefficients, Biquad};
    ///
    /// fn main() {
    ///     // Notch boost filter.
    ///     let co = Coefficients {
    ///         b0: 1.0469127398708575_f64,
    ///         b1: -0.27732002669854483,
    ///         b2: 0.8588151488168104,
    ///         a1: -0.27732002669854483,
    ///         a2: 0.9057278886876682,
    ///     };
    ///
    ///     // Note that this type argument defines the format of the temporary
    ///     // values, as well as the number of channels required for input
    ///     // `Frame`s.
    ///     let mut b = Biquad::<[f64; 2], 2>::new(co);
    ///
    ///     assert_eq!(b.apply([32i8, -64]), [33, -67]);
    ///     assert_eq!(b.apply([0.1f32, -0.3]), [0.107943736, -0.32057875]);
    /// }
    /// ```
    pub fn apply<I>(&mut self, input: I) -> I
    where
        I: Frame<N>,
        I::Sample: Duplex<F::Sample>,
    {
        // Convert into floating point representation.
        let input: F = input.map_frame(ConvertInto::convert_into);

        // Calculate scaled inputs.
        let input_by_b0 = input.mul_amp(self.coeff.b0).into_signed_frame();
        let input_by_b1 = input.mul_amp(self.coeff.b1).into_signed_frame();
        let input_by_b2 = input.mul_amp(self.coeff.b2);

        // This is the new filtered `Frame`.
        let output: F = self.t0.add_frame(input_by_b0);

        // Calculate scaled outputs.
        // NOTE: Negative signs on the scaling factors for these.
        let output_by_neg_a1 = output.mul_amp(-self.coeff.a1).into_signed_frame();
        let output_by_neg_a2 = output.mul_amp(-self.coeff.a2).into_signed_frame();

        // Update buffers.
        self.t0 = self.t1.add_frame(input_by_b1).add_frame(output_by_neg_a1);
        self.t1 = input_by_b2.add_frame(output_by_neg_a2);

        // Convert back into the original `Frame` format.
        output.map_frame(ConvertFrom::convert_from)
    }
}
