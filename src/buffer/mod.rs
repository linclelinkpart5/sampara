pub mod fixed;

use crate::Frame;

pub use fixed::Fixed;

pub trait Buffer<const N: usize>: AsRef<[Self::Frame]> + AsMut<[Self::Frame]> {
    type Frame: Frame<N>;
}

// Would love to be able to do this, but `I` is unconstrained.
// impl<I, A> Buffer for A
// where
//     I: Copy + PartialEq + Debug,
//     A: AsRef<[Self::Item]> + AsMut<[Self::Item]>,
// {
//     type Item = I;
// }

impl<'a, F, const N: usize> Buffer<N> for &'a mut [F]
where
    F: Frame<N>,
{
    type Frame = F;
}

impl<F, const N: usize, const M: usize> Buffer<N> for [F; M]
where
    F: Frame<N>,
{
    type Frame = F;
}

impl<'a, F, const N: usize, const M: usize> Buffer<N> for &'a mut [F; M]
where
    F: Frame<N>,
{
    type Frame = F;
}

impl<F, const N: usize> Buffer<N> for Box<[F]>
where
    F: Frame<N>,
{
    type Frame = F;
}

impl<F, const N: usize> Buffer<N> for Vec<F>
where
    F: Frame<N>,
{
    type Frame = F;
}
