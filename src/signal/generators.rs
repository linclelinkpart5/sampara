use crate::Frame;
use crate::signal::Signal;

/// A [`Signal`] that yields [`Frame`]s by calling a closure for each iteration.
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
pub struct Repeat<F, const N: usize>(pub(super) F)
where
    F: Frame<N>,
;

impl<F, const N: usize> Signal<N> for Repeat<F, N>
where
    F: Frame<N>,
{
    type Frame = F;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        Some(self.0)
    }
}
