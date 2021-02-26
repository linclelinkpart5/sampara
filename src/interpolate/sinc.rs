use core::f64::consts::PI;

use num_traits::Float;

use crate::{Duplex, Frame, Sample};
use crate::buffer::{Buffer, Fixed};
use crate::interpolate::Interpolator;

trait SincOp {
    fn sinc(self) -> Self;
}

impl<X> SincOp for X
where
    X: Float,
{
    #[inline]
    fn sinc(self) -> Self {
        if self.is_zero() {
            Self::one()
        } else {
            self.sin() / self
        }
    }
}

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

    #[inline]
    fn depth(&self) -> usize {
        self.buffer.capacity() / 2
    }
}

impl<F, B, const N: usize> Interpolator<N> for Sinc<F, B, N>
where
    B: Buffer<Item = F>,
    F: Frame<N>,
    F::Sample: Duplex<f64>,
{
    type Frame = F;

    fn interpolate(&self, x: f64) -> Self::Frame {
        let phil = x;
        let phir = 1.0 - x;
        let nl = self.idx;
        let nr = self.idx + 1;
        let depth = self.depth();

        let rightmost = nl + depth;
        let leftmost = nr as isize - depth as isize;
        let max_depth = if rightmost >= self.buffer.capacity() {
            self.buffer.capacity() - depth
        } else if leftmost < 0 {
            (depth as isize + leftmost) as usize
        } else {
            depth
        };

        (0..max_depth).fold(Self::Frame::EQUILIBRIUM, |mut v, n| {
            v = {
                let a = PI * (phil + n as f64);
                let first = a.sinc();
                let second = 0.5 + 0.5 * (a / depth as f64).cos();
                v.zip_apply(self.buffer[nl - n], |vs, r_lag| {
                    Sample::add_amp(
                        vs,
                        (first * second * r_lag.into_sample::<f64>())
                            .into_sample::<F::Sample>()
                            .into_signed_sample(),
                    )
                })
            };

            let a = PI * (phir + n as f64);
            let first = a.sinc();
            let second = 0.5 + 0.5 * (a / depth as f64).cos();
            v.zip_apply(self.buffer[nr + n], |vs, r_lag| {
                Sample::add_amp(
                    vs,
                    (first * second * r_lag.into_sample::<f64>())
                        .into_sample::<F::Sample>()
                        .into_signed_sample(),
                )
            })
        })
    }

    fn advance(&mut self, next_frame: Self::Frame) {
        let _prev_frame = self.buffer.push(next_frame);
        if self.idx < self.depth() {
            self.idx += 1;
        }
    }
}
