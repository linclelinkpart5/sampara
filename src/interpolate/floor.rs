use crate::Frame;
use crate::interpolate::Interpolator;

/// An [`Interpolator`] that rounds down values to 0.0, returning the previous
/// [`Frame`] from the source.
pub struct Floor<F, const N: usize>
where
    F: Frame<N>,
{
    left: F,
}
