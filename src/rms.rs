use num_traits::Float;

use crate::{Frame, Sample};
use crate::buffer::{Fixed, Storage};

/// Keeps a running RMS (root mean square) over a window of [`Frame`]s over
/// time.
#[derive(Clone)]
pub struct Rms<F, S, const N: usize>
where
    F: Frame<N>,
    S: Storage<Item = F::Float>,
{
    window: Fixed<S>,
    square_sum: F::Float,
}

impl<F, S, const N: usize> Rms<F, S, N>
where
    F: Frame<N>,
    S: Storage<Item = F::Float>,
{
    /// Creates a new [`Rms`] using a given [`Fixed`] ring buffer as a window.
    /// The initial contents of the buffer will be overwritten with
    /// equilibrium values.
    pub fn new(buffer: Fixed<S>) -> Self {
        Self {
            window: buffer,
            square_sum: Frame::EQUILIBRIUM,
        }
    }

    pub fn reset(&mut self) {
        for frame_sq in self.window.iter_mut() {
            *frame_sq = Frame::EQUILIBRIUM;
        }

        self.square_sum = Frame::EQUILIBRIUM;
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.window.capacity()
    }

    #[inline]
    pub fn next(&mut self, new_frame: F) -> F::Float {
        self.next_squared(new_frame).apply(Float::sqrt)
    }

    #[inline]
    pub fn next_squared(&mut self, new_frame: F) -> F::Float {
        // Determine the square of the new frame.
        let new_frame_square = new_frame.into_float_frame().apply(|s| s * s);

        // Push back the new frame_square.
        let removed_frame_square = self.window.push(new_frame_square);

        // Add the new frame square and subtract the removed frame square.
        self.square_sum =
            self.square_sum
                .add_frame(new_frame_square.into_signed_frame())
                .zip_apply(removed_frame_square, |s, r| {
                    // In case of floating point rounding errors, floor at
                    // equilibrium.
                    (s - r).max(Sample::EQUILIBRIUM)
                });

        self.calc_rms_squared()
    }

    pub fn current(&self) -> F::Float {
        self.calc_rms_squared().apply(Float::sqrt)
    }

    fn calc_rms_squared(&self) -> F::Float {
        let num_frames_f = Sample::from_sample(self.window.capacity() as f32);
        self.square_sum.apply(|s| s / num_frames_f)
    }
}
