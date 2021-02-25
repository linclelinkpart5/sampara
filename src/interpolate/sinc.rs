use crate::{Duplex, Frame, Sample};
use crate::buffer::{Buffer, Fixed};

/// An [`Interpolator`] that uses sinc interpolation on a window of [`Frame`]s.
///
/// One of the better sample rate converters, although it uses significantly
/// more computation.
pub struct Sinc<F, B, const N: usize>
where
    B: Buffer<Item = F>,
    F: Frame<N>,
{
    buffer: Fixed<B>,
    idx: usize,
}

impl<F, B, const N: usize> Sinc<F, B, N>
where
    B: Buffer<Item = F>,
    F: Frame<N>,
{
    /// Creates a new [`Sinc`] interpolator with a given working [`Buffer`].
    ///
    /// The given [`Buffer`] should have a length that is double the desired
    /// sinc interpolation "depth".
    ///
    /// The initial contents of the [`Buffer`] will be used as padding for the
    /// interpolated signal.
    ///
    /// Panics if the length of the given [`Buffer`] is not a multiple of 2.
    pub fn new(buffer: B) -> Self {
        // TODO: Is this needed?
        assert!(buffer.as_ref().len() % 2 == 0);

        Self {
            buffer: Fixed::from(buffer),
            idx: 0,
        }
    }
}
