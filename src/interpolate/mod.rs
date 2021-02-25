pub mod floor;
pub mod linear;
pub mod sinc;

pub use floor::*;
pub use linear::*;
pub use sinc::*;

use crate::Frame;

/// Types that can interpolate between two [`Frame`]s.
///
/// Implementations should keep track of any necessary data both before and
/// after the current [`Frame`].
pub trait Interpolator<const N: usize> {
    /// The type of frame over which to iterpolate.
    type Frame: Frame<N>;

    /// Given a value in the interval [0.0, 1.0) representing the fractional
    /// position between the two interpolated [`Frame`]s, return the
    /// interpolated [`Frame`].
    fn interpolate(&self, x: f64) -> Self::Frame;

    /// To be called whenever the interpolant value steps past 1.0.
    fn advance(&mut self, next_frame: Self::Frame);
}
