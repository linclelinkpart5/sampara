use crate::sample::FloatSample;

/// Coefficients for a digital biquad filter.
///
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

pub struct Biquad<S>
where
    S: FloatSample,
{
    coeffs: Coefficients<S>,

    // Since biquad filters are second-order, we require two historical buffers.
    // This state is updated each time the filter is applied to a frame.
    t0: S,
    t1: S,
}

impl<S> Biquad<S>
where
    S: FloatSample,
{
    pub fn reset(&mut self) {
        self.t0 = S::EQUILIBRIUM;
        self.t1 = S::EQUILIBRIUM;
    }

    pub fn process(&mut self, input: S) -> S {
        // Calculate scaled inputs.
        let input_by_b0 = input * self.coeffs.b0;
        let input_by_b1 = input * self.coeffs.b1;
        let input_by_b2 = input * self.coeffs.b2;

        // This is the new filtered frame.
        let output: S = self.t0 + input_by_b0;

        // Calculate scaled outputs.
        // NOTE: Negative signs on the scaling factors for these.
        let output_by_neg_a1 = output * -self.coeffs.a1;
        let output_by_neg_a2 = output * -self.coeffs.a2;

        // Update buffers.
        self.t0 = self.t1 + input_by_b1 + output_by_neg_a1;
        self.t1 = input_by_b2 + output_by_neg_a2;

        output
    }
}
