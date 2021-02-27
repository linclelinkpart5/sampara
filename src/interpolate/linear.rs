use crate::{Duplex, Frame, Sample};
use crate::interpolate::Interpolator;

/// An [`Interpolator`] that linearly combines a left and a right [`Frame`].
///
/// ```
/// use sampara::interpolate::{Linear, Interpolator};
///
/// fn main() {
///     let linear = Linear::new([0; 4], [40, -40, 80, -80]);
///     assert_eq!(linear.interpolate(0.00), [0, 0, 0, 0]);
///     assert_eq!(linear.interpolate(0.25), [10, -10, 20, -20]);
///     assert_eq!(linear.interpolate(0.50), [20, -20, 40, -40]);
///     assert_eq!(linear.interpolate(0.75), [30, -30, 60, -60]);
/// }
/// ```
pub struct Linear<F, const N: usize>
where
    F: Frame<N>,
{
    left: F,
    right: F,
}

impl<F, const N: usize> Linear<F, N>
where
    F: Frame<N>,
{
    /// Creates a new [`Linear`] interpolator.
    pub fn new(left: F, right: F) -> Self {
        Self { left, right }
    }
}

impl<F, const N: usize> Interpolator<N> for Linear<F, N>
where
    F: Frame<N>,
    F::Sample: Duplex<f64>,
{
    type Frame = F;

    fn interpolate(&self, x: f64) -> Self::Frame {
        self.left.zip_apply(self.right, |l, r| {
            let l_f = l.into_sample::<f64>();
            let r_f = r.into_sample::<f64>();
            let diff = r_f - l_f;
            ((diff * x) + l_f).into_sample()
        })
    }

    fn advance(&mut self, next_frame: Self::Frame) {
        self.left = self.right;
        self.right = next_frame;
    }
}
