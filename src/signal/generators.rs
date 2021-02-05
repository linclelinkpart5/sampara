use std::marker::PhantomData;

use crate::{Frame, Sample};
use crate::signal::Signal;

/// A [`Signal`] that yields [`Frame`]s by calling a closure for each iteration.
/// This closure should return [`Option<Frame>`].
pub struct FromFn<F, G, const N: usize>(pub(super) G)
where
    F: Frame<N>,
    G: FnMut() -> Option<F>,
;

impl<F, G, const N: usize> Signal<N> for FromFn<F, G, N>
where
    F: Frame<N>,
    G: FnMut() -> Option<F>,
{
    type Frame = F;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        (self.0)()
    }
}

/// A [`Signal`] that yields a given [`Frame`] repeatedly forever.
pub struct Constant<F, const N: usize>(pub(super) F)
where
    F: Frame<N>,
;

impl<F, const N: usize> Signal<N> for Constant<F, N>
where
    F: Frame<N>,
{
    type Frame = F;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        Some(self.0)
    }
}

/// A [`Signal`] that yields [`Frame::EQUILIBRIUM`] forever.
pub struct Equilibrium<F, const N: usize>(pub(super) PhantomData<F>)
where
    F: Frame<N>,
;

impl<F, const N: usize> Signal<N> for Equilibrium<F, N>
where
    F: Frame<N>,
{
    type Frame = F;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        Some(F::EQUILIBRIUM)
    }
}

/// A [`Signal`] that yields zero [`Frame`]s, mainly used in combination with
/// other [`Signal`]s.
pub struct Empty<F, const N: usize>(pub(super) PhantomData<F>)
where
    F: Frame<N>,
;

impl<F, const N: usize> Signal<N> for Empty<F, N>
where
    F: Frame<N>,
{
    type Frame = F;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        None
    }
}

/// A [`Signal`] that is powered by an underlying [`Iterator`] that yields
/// [`Frame`]s.
pub struct FromFrames<I, const N: usize>(pub(super) I)
where
    I: Iterator,
    I::Item: Frame<N>,
;

impl<I, const N: usize> Signal<N> for FromFrames<I, N>
where
    I: Iterator,
    I::Item: Frame<N>,
{
    type Frame = I::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        self.0.next()
    }
}

/// A [`Signal`] that is powered by an underlying [`Iterator`] that yields
/// [`Sample`]s.
pub struct FromSamples<F, I, const N: usize>(
    pub(super) I,
    pub(super) PhantomData<F>,
)
where
    F: Frame<N, Sample = I::Item>,
    I: Iterator,
    I::Item: Sample,
;

impl<F, I, const N: usize> Signal<N> for FromSamples<F, I, N>
where
    F: Frame<N, Sample = I::Item>,
    I: Iterator,
    I::Item: Sample,
{
    type Frame = F;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        F::from_samples(&mut self.0)
    }
}
