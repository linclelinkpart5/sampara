use crate::{Frame, Sample, Duplex};
use crate::sample::FloatSample;
use crate::signal::Signal;
use crate::biquad::Filter as BQFilter;
use crate::buffer::Buffer;
use crate::interpolate::Interpolator;
use crate::rms::Rms as RmsEngine;

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

pub struct Biquad<S, const N: usize>
where
    S: Signal<N>,
    <S::Frame as Frame<N>>::Sample: FloatSample,
{
    pub(super) signal: S,
    pub(super) filter: BQFilter<S::Frame, N>,
}

impl<S, const N: usize> Signal<N> for Biquad<S, N>
where
    S: Signal<N>,
    <S::Frame as Frame<N>>::Sample: FloatSample,
{
    type Frame = S::Frame;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        Some(self.filter.apply(self.signal.next()?))
    }
}

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

enum UninitState<B: Buffer> {
    Failed,
    Waiting(B),
}

enum RmsEngineState<F, B, const N: usize>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    Active(RmsEngine<F, B, N>),
    Uninit(UninitState<B>),
}

pub(super) struct RmsState<S, B, const N: usize>
where
    S: Signal<N>,
    <S::Frame as Frame<N>>::Sample: FloatSample,
    B: Buffer<Item = S::Frame>,
{
    signal: S,
    engine_state: RmsEngineState<S::Frame, B, N>,
}

impl<S, B, const N: usize> RmsState<S, B, N>
where
    S: Signal<N>,
    <S::Frame as Frame<N>>::Sample: FloatSample,
    B: Buffer<Item = S::Frame>,
{
    pub(super) fn zeroed(signal: S, buffer: B) -> Self {
        Self {
            signal,
            engine_state: RmsEngineState::Active(RmsEngine::from(buffer)),
        }
    }

    pub(super) fn padded(signal: S, buffer: B) -> Self {
        Self {
            signal,
            engine_state: RmsEngineState::Active(RmsEngine::from_full(buffer)),
        }
    }

    pub(super) fn fill(signal: S, buffer: B) -> Self {
        Self {
            signal,
            engine_state: RmsEngineState::Uninit(UninitState::Waiting(buffer)),
        }
    }

    fn advance(&mut self, calc_root: bool) -> Option<S::Frame> {
        let signal = &mut self.signal;

        match &mut self.engine_state {
            RmsEngineState::Active(engine) => {
                let frame = self.signal.next()?;
                let output = if calc_root {
                    engine.next(frame)
                }
                else {
                    engine.next_squared(frame)
                };

                Some(output)
            },

            RmsEngineState::Uninit(ref mut uninit_state) => {
                // The window/buffer has not been initialized from the signal.
                match uninit_state {
                    UninitState::Waiting(buffer) => {
                        // Try and fill the buffer.
                        match signal.fill_buffer(buffer) {
                            // Not enough frames, set to failure state and
                            // return `None`.
                            Err(_) => {
                                *uninit_state = UninitState::Failed;
                                None
                            },

                            // The window/buffer was able to be filled, convert
                            // to active state and then re-call this method.
                            Ok(_) => {
                                // Using `Failed` as a "free" temporary dummy
                                // value.
                                let mut owned_uninit_state = UninitState::Failed;
                                std::mem::swap(&mut owned_uninit_state, uninit_state);

                                let owned_buffer = match owned_uninit_state {
                                    UninitState::Waiting(buffer) => buffer,
                                    _ => unreachable!(),
                                };

                                let engine = RmsEngine::from_full(owned_buffer);

                                // Get the current RMS value in the engine to
                                // return later.
                                let curr_rms = if calc_root {
                                    engine.current()
                                }
                                else {
                                    engine.current_squared()
                                };

                                self.engine_state = RmsEngineState::Active(engine);

                                Some(curr_rms)
                            },
                        }
                    },

                    // There were not enough frames to initially fill the
                    // window/buffer, return `None` forever.
                    UninitState::Failed => None,
                }
            },
        }
    }
}

pub struct Rms<S, B, const N: usize>(pub(super) RmsState<S, B, N>)
where
    S: Signal<N>,
    <S::Frame as Frame<N>>::Sample: FloatSample,
    B: Buffer<Item = S::Frame>,
;

impl<S, B, const N: usize> Signal<N> for Rms<S, B, N>
where
    S: Signal<N>,
    <S::Frame as Frame<N>>::Sample: FloatSample,
    B: Buffer<Item = S::Frame>,
{
    type Frame = B::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        self.0.advance(true)
    }
}

pub struct Ms<S, B, const N: usize>(pub(super) RmsState<S, B, N>)
where
    S: Signal<N>,
    <S::Frame as Frame<N>>::Sample: FloatSample,
    B: Buffer<Item = S::Frame>,
;

impl<S, B, const N: usize> Signal<N> for Ms<S, B, N>
where
    S: Signal<N>,
    <S::Frame as Frame<N>>::Sample: FloatSample,
    B: Buffer<Item = S::Frame>,
{
    type Frame = B::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        self.0.advance(false)
    }
}
