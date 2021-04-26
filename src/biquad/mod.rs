use crate::{Frame, Duplex, ConvertFrom, ConvertInto};
use crate::sample::FloatSample;

trait Inner: FloatSample {
    fn a_cap(self) -> Self;
}

impl<F: FloatSample> Inner for F {
    fn a_cap(self) -> Self {
        let ten = F::from(10.0).unwrap();
        let fourty = F::from(40.0).unwrap();
        ten.powf(self / fourty)
    }
}

pub enum Kind<P>
where
    P: FloatSample,
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
    P: FloatSample,
{
    fn into_params(self, norm_freq: P, q_factor: P) -> Params<P> {
        // Common reused values.
        let one = P::one();
        let two = one + one;
        let half = two.recip();
        let pi = P::PI();

        let omega = two * pi * norm_freq;
        let (omega_s, omega_c) = omega.sin_cos();
        let alpha = omega_s / (two * q_factor);

        let b0: P;
        let b1: P;
        let b2: P;
        let a0: P;
        let a1: P;
        let a2: P;

        match self {
            Self::Allpass => {
                b0 = one - alpha;
                b1 = -two * omega_c;
                b2 = one + alpha;
                a0 = one + alpha;
                a1 = -two * omega_c;
                a2 = one - alpha;
            },
            Self::Lowpass => {
                b0 = (one - omega_c) * half;
                b1 = one - omega_c;
                b2 = (one - omega_c) * half;
                a0 = one + alpha;
                a1 = -two * omega_c;
                a2 = one - alpha;
            },
            Self::Highpass => {
                b0 = (one + omega_c) * half;
                b1 = -(one + omega_c);
                b2 = (one + omega_c) * half;
                a0 = one + alpha;
                a1 = -two * omega_c;
                a2 = one - alpha;
            },
            Self::Bandpass => {
                b0 = omega_s * half;
                b1 = P::zero();
                b2 = -(omega_s * half);
                a0 = one + alpha;
                a1 = -two * omega_c;
                a2 = one - alpha;
            },
            Self::Notch => {
                b0 = one;
                b1 = -two * omega_c;
                b2 = one;
                a0 = one + alpha;
                a1 = -two * omega_c;
                a2 = one - alpha;
            },
            Self::Peak(db_gain) => {
                let a = db_gain.a_cap();

                b0 = one + alpha * a;
                b1 = -two * omega_c;
                b2 = one - alpha * a;
                a0 = one + alpha / a;
                a1 = -two * omega_c;
                a2 = one - alpha / a;
            },
            Self::Lowshelf(db_gain) => {
                let a = db_gain.a_cap();
                let a_p1 = a + one;
                let a_m1 = a - one;
                let sqrt_a = a.sqrt();

                b0 = a * (a_p1 - a_m1 * omega_c + two * alpha * sqrt_a);
                b1 = two * a * (a_m1 - a_p1 * omega_c);
                b2 = a * (a_p1 - a_m1 * omega_c - two * alpha * sqrt_a);
                a0 = a_p1 + a_m1 * omega_c + two * alpha * sqrt_a;
                a1 = -two * (a_m1 + a_p1 * omega_c);
                a2 = a_p1 + a_m1 * omega_c - two * alpha * sqrt_a;
            },
            Self::Highshelf(db_gain) => {
                let a = db_gain.a_cap();
                let a_p1 = a + one;
                let a_m1 = a - one;
                let sqrt_a = a.sqrt();

                b0 = a * (a_p1 + a_m1 * omega_c + two * alpha * sqrt_a);
                b1 = -two * a * (a_m1 + a_p1 * omega_c);
                b2 = a * (a_p1 + a_m1 * omega_c - two * alpha * sqrt_a);
                a0 = a_p1 - a_m1 * omega_c + two * alpha * sqrt_a;
                a1 = two * (a_m1 - a_p1 * omega_c);
                a2 = a_p1 - a_m1 * omega_c - two * alpha * sqrt_a;
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
    X: FloatSample,
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
    X: FloatSample,
{
    pub fn from_kind(kind: Kind<X>, norm_freq: X, q_factor: X) -> Self {
        kind.into_params(norm_freq, q_factor)
    }
}

/// An implementation of a digital biquad filter, using the Direct Form 2
/// Transposed (DF2T) representation.
pub struct Filter<P, const N: usize>
where
    P: FloatSample,
{
    params: Params<P>,

    // Since biquad filters are second-order, we require two historical buffers.
    // This state is updated each time the filter is applied to a frame.
    t0: [P; N],
    t1: [P; N],
}

impl<P, const N: usize> Filter<P, N>
where
    P: FloatSample,
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
    P: FloatSample,
{
    fn from(params: Params<P>) -> Self {
        Self::new(params)
    }
}
