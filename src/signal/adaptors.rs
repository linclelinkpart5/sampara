use crate::{Sample, Frame};
use crate::signal::Signal;

fn zm_helper<S, O, F, M, const N: usize, const NO: usize, const NF: usize>(
    signal_a: &mut S,
    signal_b: &mut O,
    mut func: M,
) -> Option<F>
where
    S: Signal<N>,
    O: Signal<NO>,
    M: FnMut(S::Frame, O::Frame) -> F,
    F: Frame<NF>,
{
    Some(func(signal_a.next()?, signal_b.next()?))
}

/// Maps a function to each [`Frame`] from a [`Signal`] and yields a new
/// [`Frame`].
#[derive(Clone)]
pub struct Map<S, F, M, const N: usize, const NF: usize>
where
    S: Signal<N>,
    F: Frame<NF>,
    M: FnMut(S::Frame) -> F,
{
    pub(super) signal: S,
    pub(super) func: M,
}

impl<S, F, M, const N: usize, const NF: usize> Signal<NF>
for Map<S, F, M, N, NF>
where
    S: Signal<N>,
    F: Frame<NF>,
    M: FnMut(S::Frame) -> F,
{
    type Frame = F;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        self.signal.next().map(|f| (self.func)(f))
    }
}

/// Maps a function to each pair of [`Frame`]s from two [`Signal`]s in lockstep
/// and yields a new [`Frame`].
#[derive(Clone)]
pub struct ZipMap<S, O, F, M, const N: usize, const NO: usize, const NF: usize>
where
    S: Signal<N>,
    O: Signal<NO>,
    M: FnMut(S::Frame, O::Frame) -> F,
    F: Frame<NF>,
{
    pub(super) signal_a: S,
    pub(super) signal_b: O,
    pub(super) func: M,
}

impl<S, O, M, F, const N: usize, const NO: usize, const NF: usize> Signal<NF>
for ZipMap<S, O, F, M, N, NO, NF>
where
    S: Signal<N>,
    O: Signal<NO>,
    M: FnMut(S::Frame, O::Frame) -> F,
    F: Frame<NF>,
{
    type Frame = F;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        zm_helper(&mut self.signal_a, &mut self.signal_b, &mut self.func)
    }
}

/// Adds together pairs of [`Frame`]s from two [`Signal`]s in lockstep and
/// yields their sum.
#[derive(Clone)]
pub struct AddSignal<A, B, const N: usize>
where
    A: Signal<N>,
    B: Signal<N>,
    A::Frame: Frame<N, Signed = <B::Frame as Frame<N>>::Signed>,
{
    signal_a: A,
    signal_b: B,
}

impl<A, B, const N: usize> Signal<N> for AddSignal<A, B, N>
where
    A: Signal<N>,
    B: Signal<N>,
    A::Frame: Frame<N, Signed = <B::Frame as Frame<N>>::Signed>,
{
    type Frame = A::Frame;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        // Some(self.signal_a.next()?.add_frame(self.signal_b.next()?.into_signed_frame()))
        zm_helper(
            &mut self.signal_a,
            &mut self.signal_b,
            |a, b| a.add_frame(b.into_signed_frame()),
        )
    }
}
