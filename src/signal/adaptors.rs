use crate::{Frame, Sample, Duplex, Processor, Combinator};
use crate::buffer::Buffer;
use crate::sample::FloatSample;
use crate::signal::Signal;
use crate::stats::MovingCalculator;
use crate::biquad::Biquad as BQFilter;
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

/// Creates a new [`Signal`] that yields every `N`th [`Frame`] from another
/// [`Signal`].
#[derive(Clone)]
pub struct StepBy<S, const N: usize>
where
    S: Signal<N>,
{
    signal: S,
    n: usize,
    started: bool,
}

impl<S, const N: usize> StepBy<S, N>
where
    S: Signal<N>,
{
    pub(super) fn new(signal: S, step: usize) -> Self {
        let n = step.checked_sub(1).expect("step size cannot be 0");

        Self {
            signal,
            n,
            started: false,
        }
    }
}

impl<S, const N: usize> Signal<N> for StepBy<S, N>
where
    S: Signal<N>,
{
    type Frame = S::Frame;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        if !self.started {
            self.started = true;
            self.signal.next()
        }
        else {
            self.signal.nth(self.n)
        }
    }
}

/// A [`Signal`] that processes [`Frame`]s from an input [`Signal`] with a
/// given [`Processor`] and yields the output [`Frame`]s.
pub struct Process<S, P, const NI: usize, const NO: usize>
where
    S: Signal<NI>,
    P: Processor<NI, NO, Input = S::Frame>,
{
    pub(super) signal: S,
    pub(crate) processor: P,
}

impl<S, P, const NI: usize, const NO: usize> Process<S, P, NI, NO>
where
    S: Signal<NI>,
    P: Processor<NI, NO, Input = S::Frame>,
{
    /// Returns a reference to the internal [`Processor`] state.
    pub fn state(&self) -> &P {
        &self.processor
    }

    /// Returns a mutable reference to the internal [`Processor`] state.
    pub fn state_mut(&mut self) -> &mut P {
        &mut self.processor
    }
}

impl<S, P, const NI: usize, const NO: usize> Signal<NO>
for Process<S, P, NI, NO>
where
    S: Signal<NI>,
    P: Processor<NI, NO, Input = S::Frame>,
{
    type Frame = P::Output;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        let input = self.signal.next()?;
        let output = self.processor.process(input);
        Some(output)
    }
}

/// A [`Signal`] that combines pairs of [`Frame`]s in lockstep from two input
/// [`Signal`]s with a given [`Combinator`] and yields the output [`Frame`]s.
pub struct Combine<SL, SR, C, const NL: usize, const NR: usize, const NO: usize>
where
    SL: Signal<NL>,
    SR: Signal<NR>,
    C: Combinator<NL, NR, NO, InputL = SL::Frame, InputR = SR::Frame>,
{
    pub(super) signal_l: SL,
    pub(super) signal_r: SR,
    pub(super) combinator: C,
}

impl<SL, SR, C, const NL: usize, const NR: usize, const NO: usize> Combine<SL, SR, C, NL, NR, NO>
where
    SL: Signal<NL>,
    SR: Signal<NR>,
    C: Combinator<NL, NR, NO, InputL = SL::Frame, InputR = SR::Frame>,
{
    /// Returns a reference to the internal [`Combinator`] state.
    pub fn state(&self) -> &C {
        &self.combinator
    }

    /// Returns a mutable reference to the internal [`Combinator`] state.
    pub fn state_mut(&mut self) -> &mut C {
        &mut self.combinator
    }
}

impl<SL, SR, C, const NL: usize, const NR: usize, const NO: usize> Signal<NO>
for Combine<SL, SR, C, NL, NR, NO>
where
    SL: Signal<NL>,
    SR: Signal<NR>,
    C: Combinator<NL, NR, NO, InputL = SL::Frame, InputR = SR::Frame>,
{
    type Frame = C::Output;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        let input_l = self.signal_l.next()?;
        let input_r = self.signal_r.next()?;
        let output = self.combinator.combine(input_l, input_r);
        Some(output)
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
        Some(self.filter.process(self.signal.next()?))
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

stats_inject_signal_adaptors!();
