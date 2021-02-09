use crate::{Frame, Duplex, ConvertFrom, ConvertInto};
use crate::sample::FloatSample;

enum Kind {
    Lowpass,
    Highpass,
    Bandpass,
    Notch,
    Peak,
    Lowshelf,
    Highshelf,
}

/// Coefficients for a digital biquad filter.
/// It is assumed that the `a0` coefficient is always normalized to 1.0,
/// and thus not included.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Coefficients<X>
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

impl<X> Coefficients<X>
where
    X: FloatSample + From<f64>,
{
    fn from_kind(kind: Kind, norm_freq: X, q_factor: X, peak_gain: X) -> Self {
        let one: X = X::one();
        let two: X = 2.0.into();
        let ten: X = 10.0.into();
        let pi: X = std::f64::consts::PI.into();

        let v = ten.powf(peak_gain.abs() / 20.0.into());
        let k = (pi * norm_freq).tan();
        let k_sq = k * k;
        let k_by_q = k / q_factor;

        let b0: X;
        let b1: X;
        let b2: X;
        let a1: X;
        let a2: X;

        match kind {
            Kind::Lowpass => {
                let norm = one / (one + k_by_q + k_sq);

                b0 = k_sq * norm;
                b1 = two * b0;
                b2 = b0;
                a1 = two * (k_sq - one) * norm;
                a2 = (one - k_by_q + k_sq) * norm;
            },

            Kind::Highpass => {
                let norm = one / (one + k_by_q + k_sq);

                b0 = one * norm;
                b1 = -two * b0;
                b2 = b0;
                a1 = two * (k_sq - one) * norm;
                a2 = (one - k_by_q + k_sq) * norm;
            },

            Kind::Bandpass => {
                let norm = one / (one + k_by_q + k_sq);

                b0 = k_by_q * norm;
                b1 = X::zero();
                b2 = -b0;
                a1 = two * (k_sq - one) * norm;
                a2 = (one - k_by_q + k_sq) * norm;
            },

            Kind::Notch => {
                let norm = one / (one + k_by_q + k_sq);

                b0 = (one + k_sq) * norm;
                b1 = two * (k_sq - one) * norm;
                b2 = b0;
                a1 = b1;
                a2 = (one - k_by_q + k_sq) * norm;
            },

            Kind::Peak => {
                // Peak boost.
                if peak_gain >= X::zero() {
                    let norm = one / (one + k_by_q + k_sq);

                    b0 = (one + v * k_by_q + k_sq) * norm;
                    b1 = two * (k_sq - one) * norm;
                    b2 = (one - v * k_by_q + k_sq) * norm;
                    a1 = b1;
                    a2 = (one - k_by_q + k_sq) * norm;
                }
                // Peak cut.
                else {
                    let norm = one / (one + v * k_by_q + k_sq);

                    b0 = (one + k_by_q + k_sq) * norm;
                    b1 = two * (k_sq - one) * norm;
                    b2 = (one - k_by_q + k_sq) * norm;
                    a1 = b1;
                    a2 = (one - v * k_by_q + k_sq) * norm;
                }
            },
            Kind::Lowshelf => {
                let sqrt2: X = std::f64::consts::SQRT_2.into();
                let sqrt2v: X = sqrt2 * v.sqrt();

                // Boost shelf.
                if peak_gain >= X::zero() {
                    let norm = one / (one + sqrt2 * k + k_sq);

                    b0 = (one + sqrt2v * k + v * k_sq) * norm;
                    b1 = two * (v * k_sq - one) * norm;
                    b2 = (one - sqrt2v * k + v * k_sq) * norm;
                    a1 = two * (k_sq - one) * norm;
                    a2 = (one - sqrt2 * k + k_sq) * norm;
                }
                // Cut shelf.
                else {
                    let norm = one / (one + sqrt2v * k + v * k_sq);

                    b0 = (one + sqrt2 * k + k_sq) * norm;
                    b1 = two * (k_sq - one) * norm;
                    b2 = (one - sqrt2 * k + k_sq) * norm;
                    a1 = two * (v * k_sq - one) * norm;
                    a2 = (one - sqrt2v * k + v * k_sq) * norm;
                }
            },
            Kind::Highshelf => {
                let sqrt2: X = std::f64::consts::SQRT_2.into();
                let sqrt2v: X = sqrt2 * v.sqrt();

                // Boost shelf.
                if peak_gain >= X::zero() {
                    let norm = one / (one + sqrt2 * k + k_sq);

                    b0 = (v + sqrt2v * k + k_sq) * norm;
                    b1 = two * (k_sq - v) * norm;
                    b2 = (v - sqrt2v * k + k_sq) * norm;
                    a1 = two * (k_sq - one) * norm;
                    a2 = (one - sqrt2 * k + k_sq) * norm;
                }
                // Cut shelf.
                else {
                    let norm = one / (v + sqrt2v * k + k_sq);

                    b0 = (one + sqrt2 * k + k_sq) * norm;
                    b1 = two * (k_sq - one) * norm;
                    b2 = (one - sqrt2 * k + k_sq) * norm;
                    a1 = two * (k_sq - v) * norm;
                    a2 = (v - sqrt2v * k + k_sq) * norm;
                }
            },
        };

        Coefficients { b0, b1, b2, a1, a2 }
    }
}

/// An implementation of a digital biquad filter, using the Direct Form 2
/// Transposed (DF2T) representation.
pub struct Biquad<F, const N: usize>
where
    F: Frame<N>,
    F::Sample: FloatSample,
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
    F::Sample: FloatSample,
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
