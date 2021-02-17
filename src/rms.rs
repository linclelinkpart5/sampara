use crate::Frame;
use crate::buffer::{Fixed, Storage};

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
}
