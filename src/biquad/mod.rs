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
    fn into_params(self, norm_freq: P, q_factor: P) -> Params<P> {
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

        Params {
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
pub struct Params<X>
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

impl<X> Params<X>
where
    X: Param,
{
    pub fn from_kind(kind: Kind<X>, norm_freq: X, q_factor: X) -> Self {
        kind.into_params(norm_freq, q_factor)
    }
}

/// An implementation of a digital biquad filter, using the Direct Form 2
/// Transposed (DF2T) representation.
pub struct Filter<P, const N: usize>
where
    P: FloatSample + Param,
{
    params: Params<P>,

    // Since biquad filters are second-order, we require two historical buffers.
    // This state is updated each time the filter is applied to a frame.
    t0: [P; N],
    t1: [P; N],
}

impl<P, const N: usize> Filter<P, N>
where
    P: FloatSample + Param,
{
    pub fn new(params: Params<P>) -> Self {
        Self {
            params,
            t0: Frame::EQUILIBRIUM,
            t1: Frame::EQUILIBRIUM,
        }
    }

    /// Performs a single iteration of this filter, calculating a new filtered
    /// `Frame` from an input `Frame`.
    ///
    /// ```
    /// use sampara::biquad::{Kind, Params, Filter};
    ///
    /// fn main() {
    ///     // Notch filter.
    ///     let params = Params::from_kind(Kind::Notch, 0.25, 0.7071);
    ///
    ///     let inputs = &[
    ///         [-57,  61], [ 50,  13], [  5,  91], [-16,  -7],
    ///         [ 74, -36], [ 85, -37], [-48,  19], [-64,  -8],
    ///         [  1,  77], [ 28,  45], [ 83,  47], [-34, -92],
    ///         [ 16,   4], [ 74,  45], [-89,   5], [-63, -53],
    ///     ];
    ///
    ///     let expected = &[
    ///         [-33,  35], [ 29,   7], [-24,  82], [ 14,   2],
    ///         [ 50,  17], [ 37, -26], [  6, -13], [  5, -21],
    ///         [-28,  58], [-22,  25], [ 54,  62], [  0, -31],
    ///         [ 48,  19], [ 23, -22], [-51,   1], [  2,   0],
    ///     ];
    ///
    ///     // Note that this type argument defines the format of the temporary
    ///     // values, as well as the number of channels required for input
    ///     // `Frame`s.
    ///     let mut filter = Filter::<f64, 2>::new(params);
    ///
    ///     let mut produced = vec![];
    ///     for &input in inputs.iter() {
    ///         produced.push(filter.apply(input));
    ///     }
    ///
    ///     assert_eq!(&produced, expected);
    /// }
    /// ```
    pub fn apply<I>(&mut self, input: I) -> I
    where
        I: Frame<N>,
        I::Sample: Duplex<P>,
    {
        // Convert into floating point representation.
        let input: [P; N] = input.apply(ConvertInto::convert_into);

        // Calculate scaled inputs.
        let input_by_b0 = input.mul_amp(self.params.b0).into_signed_frame();
        let input_by_b1 = input.mul_amp(self.params.b1).into_signed_frame();
        let input_by_b2 = input.mul_amp(self.params.b2);

        // This is the new filtered `Frame`.
        let output: [P; N] = self.t0.add_frame(input_by_b0);

        // Calculate scaled outputs.
        // NOTE: Negative signs on the scaling factors for these.
        let output_by_neg_a1 = output.mul_amp(-self.params.a1).into_signed_frame();
        let output_by_neg_a2 = output.mul_amp(-self.params.a2).into_signed_frame();

        // Update buffers.
        self.t0 = self.t1.add_frame(input_by_b1).add_frame(output_by_neg_a1);
        self.t1 = input_by_b2.add_frame(output_by_neg_a2);

        // Convert back into the original `Frame` format.
        output.apply(ConvertFrom::convert_from)
    }
}

impl<P, const N: usize> From<Params<P>> for Filter<P, N>
where
    P: FloatSample + Param,
{
    fn from(params: Params<P>) -> Self {
        Self::new(params)
    }
}
