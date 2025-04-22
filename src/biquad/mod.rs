use crate::{frame::Frame, sample::FloatSample};

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

pub struct Biquad<F>
where
    F: Frame,
    F::Sample: FloatSample,
{
    coeffs: Coefficients<F::Sample>,

    // Since biquad filters are second-order, we require two historical buffers.
    // This state is updated each time the filter is applied to a frame.
    t0: F,
    t1: F,
}

impl<F> Biquad<F>
where
    F: Frame,
    F::Sample: FloatSample,
{
    pub fn reset(&mut self) {
        self.t0 = Frame::equil();
        self.t1 = Frame::equil();
    }

    // pub fn process(&mut self, input: F) -> F {
    //     // Calculate scaled inputs.
    //     let input_by_b0 = input.mul_amp(self.coeffs.b0).into_signed_frame();
    //     let input_by_b1 = input.mul_amp(self.coeffs.b1).into_signed_frame();
    //     let input_by_b2 = input.mul_amp(self.coeffs.b2);

    //     // This is the new filtered frame.
    //     let output: F = self.t0.add_frame(input_by_b0);

    //     // Calculate scaled outputs.
    //     // NOTE: Negative signs on the scaling factors for these.
    //     let output_by_neg_a1 = output.mul_amp(-self.coeffs.a1).into_signed_frame();
    //     let output_by_neg_a2 = output.mul_amp(-self.coeffs.a2).into_signed_frame();

    //     // Update buffers.
    //     self.t0 = self.t1.add_frame(input_by_b1).add_frame(output_by_neg_a1);
    //     self.t1 = input_by_b2.add_frame(output_by_neg_a2);

    //     output
    // }
}
