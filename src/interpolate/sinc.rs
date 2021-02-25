use core::f64::consts::PI;

use crate::{Duplex, Frame, Sample};
use crate::buffer::{Buffer, Fixed};
use crate::interpolate::Interpolator;

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
        // let phil = x;
        // let phir = 1.0 - x;
        // let nl = self.idx;
        // let nr = self.idx + 1;
        // let depth = self.depth();

        // let rightmost = nl + depth;
        // let leftmost = nr as isize - depth as isize;
        // let max_depth = if rightmost >= self.frames.len() {
        //     self.frames.len() - depth
        // } else if leftmost < 0 {
        //     (depth as isize + leftmost) as usize
        // } else {
        //     depth
        // };

        // (0..max_depth).fold(Self::Frame::EQUILIBRIUM, |mut v, n| {
        //     v = {
        //         let a = PI * (phil + n as f64);
        //         let first = if a == 0.0 { 1.0 } else { sin(a) / a };
        //         let second = 0.5 + 0.5 * cos(a / depth as f64);
        //         v.zip_map(self.frames[nl - n], |vs, r_lag| {
        //             vs.add_amp(
        //                 (first * second * r_lag.to_sample::<f64>())
        //                     .to_sample::<<Self::Frame as Frame>::Sample>()
        //                     .to_signed_sample(),
        //             )
        //         })
        //     };

        //     let a = PI * (phir + n as f64);
        //     let first = if a == 0.0 { 1.0 } else { sin(a) / a };
        //     let second = 0.5 + 0.5 * cos(a / depth as f64);
        //     v.zip_map(self.frames[nr + n], |vs, r_lag| {
        //         vs.add_amp(
        //             (first * second * r_lag.to_sample::<f64>())
        //                 .to_sample::<<Self::Frame as Frame>::Sample>()
        //                 .to_signed_sample(),
        //         )
        //     })
        // })

        todo!()
    }

    fn advance(&mut self, next_frame: Self::Frame) {
        let _prev_frame = self.buffer.push(next_frame);
        if self.idx < self.depth() {
            self.idx += 1;
        }
    }
}
