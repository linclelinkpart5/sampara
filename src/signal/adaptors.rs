use crate::{Frame, Sample};
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
    pub(super) signal_a: A,
    pub(super) signal_b: B,
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
        zm_helper(
            &mut self.signal_a,
            &mut self.signal_b,
            |a, b| a.add_frame(b.into_signed_frame()),
        )
    }
}

/// Multiplies together pairs of [`Frame`]s from two [`Signal`]s in lockstep and
/// yields their product.
#[derive(Clone)]
pub struct MulSignal<A, B, const N: usize>
where
    A: Signal<N>,
    B: Signal<N>,
    A::Frame: Frame<N, Float = <B::Frame as Frame<N>>::Float>,
{
    pub(super) signal_a: A,
    pub(super) signal_b: B,
}

impl<A, B, const N: usize> Signal<N> for MulSignal<A, B, N>
where
    A: Signal<N>,
    B: Signal<N>,
    A::Frame: Frame<N, Float = <B::Frame as Frame<N>>::Float>,
{
    type Frame = A::Frame;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        zm_helper(
            &mut self.signal_a,
            &mut self.signal_b,
            |a, b| a.mul_frame(b.into_float_frame()),
        )
    }
}

/// Adds a constant [`Frame`] to each [`Frame`] from a [`Signal`].
#[derive(Clone)]
pub struct AddFrame<S, F, const N: usize>
where
    S: Signal<N>,
    S::Frame: Frame<N, Signed = F>,
    F: Frame<N>,
{
    pub(super) signal: S,
    pub(super) frame: F,
}

impl<S, F, const N: usize> Signal<N> for AddFrame<S, F, N>
where
    S: Signal<N>,
    S::Frame: Frame<N, Signed = F>,
    F: Frame<N>,
{
    type Frame = S::Frame;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        Some(self.signal.next()?.add_frame(self.frame))
    }
}

/// Multiplies a constant [`Frame`] to each [`Frame`] from a [`Signal`].
#[derive(Clone)]
pub struct MulFrame<S, F, const N: usize>
where
    S: Signal<N>,
    S::Frame: Frame<N, Float = F>,
    F: Frame<N>,
{
    pub(super) signal: S,
    pub(super) frame: F,
}

impl<S, F, const N: usize> Signal<N> for MulFrame<S, F, N>
where
    S: Signal<N>,
    S::Frame: Frame<N, Float = F>,
    F: Frame<N>,
{
    type Frame = S::Frame;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        Some(self.signal.next()?.mul_frame(self.frame))
    }
}

/// Adds a constant [`Sample`] to each channel in each [`Frame`] from a
/// [`Signal`].
#[derive(Clone)]
pub struct AddAmp<S, X, const N: usize>
where
    S: Signal<N>,
    S::Frame: Frame<N>,
    <S::Frame as Frame<N>>::Sample: Sample<Signed = X>,
    X: Sample,
{
    pub(super) signal: S,
    pub(super) amp: X,
}

impl<S, X, const N: usize> Signal<N> for AddAmp<S, X, N>
where
    S: Signal<N>,
    S::Frame: Frame<N>,
    <S::Frame as Frame<N>>::Sample: Sample<Signed = X>,
    X: Sample,
{
    type Frame = S::Frame;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        Some(self.signal.next()?.add_amp(self.amp))
    }
}

/// Multiplies a constant [`Sample`] to each channel in each [`Frame`] from a
/// [`Signal`].
#[derive(Clone)]
pub struct MulAmp<S, X, const N: usize>
where
    S: Signal<N>,
    S::Frame: Frame<N>,
    <S::Frame as Frame<N>>::Sample: Sample<Float = X>,
    X: Sample,
{
    pub(super) signal: S,
    pub(super) amp: X,
}

impl<S, X, const N: usize> Signal<N> for MulAmp<S, X, N>
where
    S: Signal<N>,
    S::Frame: Frame<N>,
    <S::Frame as Frame<N>>::Sample: Sample<Float = X>,
    X: Sample,
{
    type Frame = S::Frame;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        Some(self.signal.next()?.mul_amp(self.amp))
    }
}
