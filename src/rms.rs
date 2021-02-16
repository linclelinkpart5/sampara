use crate::Frame;
use crate::buffer::{Fixed, Storage};

#[derive(Clone)]
pub struct Rms<F, S, const N: usize>
where
    F: Frame<N>,
    S: Storage<F::Float>,
{
    window: Fixed<F::Float, S>,
    square_sum: F::Float,
}

impl<F, S, const N: usize> Rms<F, S, N>
where
    F: Frame<N>,
    S: Storage<F::Float>,
{
    pub fn new(buffer: Fixed<F::Float, S>) -> Self {
        Self {
            window: buffer,
            square_sum: Frame::EQUILIBRIUM,
        }
    }
}
