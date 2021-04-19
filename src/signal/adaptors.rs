use crate::{Frame, Sample};
use crate::signal::Signal;
#[cfg(feature = "biquad")]
use crate::{Duplex, biquad::{Param, Filter as BQFilter}, sample::FloatSample};
#[cfg(feature = "interpolate")]
use crate::interpolate::Interpolator;

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

/// Delays a [`Signal`] a given number of [`Frame`]s by yielding
/// [`Frame::EQUILIBRIUM`] that many times before yielding from the contained
/// [`Signal`].
#[derive(Clone)]
pub struct Delay<S, const N: usize>
where
    S: Signal<N>,
{
    pub(super) signal: S,
    pub(super) n_frames: usize,
}

impl<S, const N: usize> Signal<N> for Delay<S, N>
where
    S: Signal<N>,
{
    type Frame = S::Frame;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        if self.n_frames > 0 {
            self.n_frames -= 1;
            Some(Frame::EQUILIBRIUM)
        } else {
            self.signal.next()
        }
    }
}

/// Creates a new [`Signal`] that calls a function with each [`Frame`], and then
/// yields the [`Frame`].
#[derive(Clone)]
pub struct Inspect<S, F, const N: usize>
where
    S: Signal<N>,
    F: FnMut(&S::Frame),
{
    pub(super) signal: S,
    pub(super) func: F,
}

impl<S, F, const N: usize> Signal<N> for Inspect<S, F, N>
where
    S: Signal<N>,
    F: FnMut(&S::Frame),
{
    type Frame = S::Frame;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        self.signal.next().map(|f| {
            (self.func)(&f);
            f
        })
    }
}

/// Creates a new [`Signal`] that yields the first N [`Frame`]s of a [`Signal`],
/// and then stops.
#[derive(Clone)]
pub struct Take<S, const N: usize>
where
    S: Signal<N>,
{
    pub(super) signal: S,
    pub(super) n: usize,
}

impl<S, const N: usize> Signal<N> for Take<S, N>
where
    S: Signal<N>,
{
    type Frame = S::Frame;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        if self.n > 0 {
            self.n -= 1;
            self.signal.next()
        }
        else {
            None
        }
    }
}

/// Creates a new [`Signal`] that yields all of the [`Frame`]s from another
/// [`Signal`]. If the [`Signal`] yields less than N [`Frame`]s, then this will
/// yield [`Frame::EQUILIBRIUM`] until N total [`Frame`]s have been yielded.
#[derive(Clone)]
pub struct Pad<S, const N: usize>
where
    S: Signal<N>,
{
    pub(super) signal: S,
    pub(super) n: usize,
}

impl<S, const N: usize> Signal<N> for Pad<S, N>
where
    S: Signal<N>,
{
    type Frame = S::Frame;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        let ret = match self.signal.next() {
            None if self.n > 0 => Some(Frame::EQUILIBRIUM),
            x => x,
        };

        self.n = self.n.saturating_sub(1);

        ret
    }
}

#[cfg(feature = "biquad")]
pub struct Biquad<S, P, const N: usize>
where
    S: Signal<N>,
    P: Param + FloatSample,
    <S::Frame as Frame<N>>::Sample: Duplex<P>,
{
    pub(super) signal: S,
    pub(super) filter: BQFilter<P, N>,
}

#[cfg(feature = "biquad")]
impl<S, P, const N: usize> Signal<N> for Biquad<S, P, N>
where
    S: Signal<N>,
    P: Param + FloatSample,
    <S::Frame as Frame<N>>::Sample: Duplex<P>,
{
    type Frame = S::Frame;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        Some(self.filter.apply(self.signal.next()?))
    }
}

#[cfg(feature = "interpolate")]
pub struct Interpolate<S, I, const N: usize>
where
    S: Signal<N>,
    I: Interpolator<N, Frame = S::Frame>,
    <S::Frame as Frame<N>>::Sample: Duplex<f64>,
{
    pub(super) signal: S,
    pub(super) interpolator: I,
    pub(super) interpolant: f64,
    pub(super) step: f64,
    pub(super) end_padding: Option<S::Frame>,
}

#[cfg(feature = "interpolate")]
impl<S, I, const N: usize> Signal<N> for Interpolate<S, I, N>
where
    S: Signal<N>,
    I: Interpolator<N, Frame = S::Frame>,
    <S::Frame as Frame<N>>::Sample: Duplex<f64>,
{
    type Frame = I::Frame;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        let Interpolate {
            ref mut signal,
            ref mut interpolator,
            ref mut interpolant,
            step,
            ref mut end_padding,
        } = *self;

        // Advance frames.
        while *interpolant >= 1.0 {
            interpolator.advance(signal.next().or_else(|| end_padding.take())?);
            *interpolant -= 1.0;
        }

        let out = interpolator.interpolate(*interpolant);
        *interpolant += step;
        Some(out)
    }
}
