use crate::frame::{Dynamic, Fixed, Frame};
use crate::sample::Sample;
use crate::signal::Signal;

/// A [`Signal`] that yields [`Frame`]s by calling a closure for each iteration.
/// This closure should return [`Option<Frame>`].
pub struct FromFn<F, G>(pub(super) G)
where
    F: Frame,
    G: FnMut() -> Option<F>;

impl<F, G> Signal for FromFn<F, G>
where
    F: Frame,
    G: FnMut() -> Option<F>,
{
    type Frame = F;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        (self.0)()
    }
}

/// A [`Signal`] that is powered by an underlying [`Iterator`] that yields
/// [`Frame`]s.
pub struct FromFrames<I>(pub(super) I)
where
    I: Iterator,
    I::Item: Frame;

impl<I> Signal for FromFrames<I>
where
    I: Iterator,
    I::Item: Frame,
{
    type Frame = I::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        self.0.next()
    }
}

/// A [`Signal`] that is powered by an underlying [`Iterator`] that yields
/// [`Sample`]s. This [`Signal`] yields fixed-size [`Frame`]s.
pub struct FromSamplesFixed<I, const N: usize>(pub(super) I)
where
    I: Iterator,
    I::Item: Sample;

impl<I, const N: usize> Signal for FromSamplesFixed<I, N>
where
    I: Iterator,
    I::Item: Sample,
{
    type Frame = Fixed<I::Item, N>;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        Fixed::from_samples(&mut self.0)
    }
}

/// A [`Signal`] that is powered by an underlying [`Iterator`] that yields
/// [`Sample`]s. This [`Signal`] yields fixed-size [`Frame`]s.
pub struct FromSamplesDynamic<I>(pub(super) I, pub(super) usize)
where
    I: Iterator,
    I::Item: Sample;

impl<I> Signal for FromSamplesDynamic<I>
where
    I: Iterator,
    I::Item: Sample,
{
    type Frame = Dynamic<I::Item>;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        Dynamic::from_samples(&mut self.0, self.1)
    }
}