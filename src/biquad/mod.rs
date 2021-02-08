use crate::{Frame, Duplex, ConvertFrom, ConvertInto};
use crate::sample::FloatSample;

/// Biquad filter kinds that use `K`, `K^2`, and `K/Q`.
#[derive(Clone, Copy)]
enum NormKind {
    LowPass,
    HighPass,
    BandPass,
    Notch,
}

impl NormKind {
    fn params<X>(self, norm_freq: X, q_factor: X) -> Coefficients<X>
    where
        X: FloatSample + From<f64>,
    {
        let one: X = X::one();
        let two: X = X::one() + X::one();
        let pi: X = std::f64::consts::PI.into();

        let k = (pi * norm_freq).tan();
        let k_sq = k * k;
        let k_by_q = k / q_factor;
        let pinv_norm = one + k_by_q + k_sq;
        let ninv_norm = one - k_by_q + k_sq;
        let k_sq_m1 = k_sq - one;
        let k_sq_p1 = k_sq + one;

        match self {
            Self::LowPass => Coefficients {
                b0: pinv_norm / k_sq,
                b1: two * k_sq_m1 / k_sq,
                b2: ninv_norm / k_sq,
                a1: two,
                a2: one,
            },
            Self::HighPass => Coefficients {
                b0: pinv_norm,
                b1: two * k_sq_m1,
                b2: ninv_norm,
                a1: -two,
                a2: one,
            },
            Self::BandPass => Coefficients {
                b0: pinv_norm / k_by_q,
                b1: two * k_sq_m1 / k_by_q,
                b2: ninv_norm / k_by_q,
                a1: X::zero(),
                a2: -one,
            },
            Self::Notch => Coefficients {
                b0: pinv_norm / k_sq_p1,
                b1: two * k_sq_m1 / k_sq_p1,
                b2: ninv_norm / k_sq_p1,
                a1: two * k_sq_m1 / k_sq_p1,
                a2: one,
            },
        }
    }
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
    pub fn allpass(norm_freq: X, q_factor: X) -> Self {
        let one: X = 1.0.into();
        let two: X = 2.0.into();

        let alpha = norm_freq.sin() / two * q_factor;
        let cs = norm_freq.cos();

        let b0 = one / (one - alpha);
        let b1 = -two * cs * b0;

        Self {
            b0,
            b1,
            b2: one,
            a1: b1,
            a2: (one + alpha) * b0,
        }
    }

    pub fn lowpass(norm_freq: X, q_factor: X) -> Self {
        NormKind::LowPass.params(norm_freq, q_factor)
    }

    pub fn highpass(norm_freq: X, q_factor: X) -> Self {
        NormKind::HighPass.params(norm_freq, q_factor)
    }

    pub fn bandpass(norm_freq: X, q_factor: X) -> Self {
        NormKind::BandPass.params(norm_freq, q_factor)
    }

    pub fn notch(norm_freq: X, q_factor: X) -> Self {
        NormKind::Notch.params(norm_freq, q_factor)
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
