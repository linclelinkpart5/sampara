use crate::{Duplex, Frame, Signal};
use crate::interpolate::Interpolator;

/// An [`Interpolator`] that rounds down to the previous source [`Frame`].
///
/// ```
/// use sampara::interpolate::{Floor, Interpolator};
///
/// fn main() {
///     let floor = Floor::new([0, 1, 2, 3]);
///     assert_eq!(floor.interpolate(0.00), [0, 1, 2, 3]);
///     assert_eq!(floor.interpolate(0.25), [0, 1, 2, 3]);
///     assert_eq!(floor.interpolate(0.50), [0, 1, 2, 3]);
///     assert_eq!(floor.interpolate(0.75), [0, 1, 2, 3]);
/// }
/// ```
pub struct Floor<F, const N: usize>
where
    F: Frame<N>,
{
    left: F,
}

impl<F, const N: usize> Floor<F, N>
where
    F: Frame<N>,
{
    /// Creates a new [`Floor`] interpolator.
    pub fn new(left: F) -> Self {
        Self { left }
    }
}

impl<F, const N: usize> Interpolator<N> for Floor<F, N>
where
    F: Frame<N>,
    F::Sample: Duplex<f64>,
{
    type Frame = F;

    fn interpolate(&self, _x: f64) -> Self::Frame {
        self.left
    }

    fn advance(&mut self, next_frame: Self::Frame) {
        self.left = next_frame;
    }

    fn initialize<S>(&mut self, signal: &mut S) -> Option<()>
    where
        S: Signal<N, Frame = F>
    {
        *self = Self {
            left: signal.next()?,
        };

        Some(())
    }
}
