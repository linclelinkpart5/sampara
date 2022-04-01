use num_traits::Float;

use crate::sample::FloatSample;
use crate::{Frame, Signal};

// LEARN: Good example of the difference between type generics and associated
//        types.
// pub trait OldStep<F, const N: usize>
// where
//     F: Frame<N>,
//     F::Sample: FloatSample,
// {
//     fn step(&mut self) -> Option<F>;
// }

pub trait Step<X, const N: usize>
where
    X: FloatSample,
{
    type Step: Frame<N, Sample = X>;

    fn step(&mut self) -> Option<Self::Step>;
}

pub struct Fixed<F, const N: usize>(F)
where
    F: Frame<N>,
    F::Sample: FloatSample;

impl<F, const N: usize> Step<F::Sample, N> for Fixed<F, N>
where
    F: Frame<N>,
    F::Sample: FloatSample,
{
    type Step = F;

    fn step(&mut self) -> Option<Self::Step> {
        Some(self.0)
    }
}

enum VarInner<S, const N: usize>
where
    S: Signal<N>,
    <S::Frame as Frame<N>>::Sample: FloatSample,
{
    Hzs(S, <S::Frame as Frame<N>>::Sample),
    Steps(S),
}

impl<S, const N: usize> Step<<S::Frame as Frame<N>>::Sample, N> for VarInner<S, N>
where
    S: Signal<N>,
    <S::Frame as Frame<N>>::Sample: FloatSample,
{
    type Step = S::Frame;

    fn step(&mut self) -> Option<Self::Step> {
        match self {
            Self::Hzs(hz_signal, rate) => hz_signal.next().map(|f| f.mul_amp(rate.recip())),
            Self::Steps(steps_signal) => steps_signal.next(),
        }
    }
}

pub struct Variable<S, const N: usize>(VarInner<S, N>)
where
    S: Signal<N>,
    <S::Frame as Frame<N>>::Sample: FloatSample;

impl<S, const N: usize> Step<<S::Frame as Frame<N>>::Sample, N> for Variable<S, N>
where
    S: Signal<N>,
    <S::Frame as Frame<N>>::Sample: FloatSample,
{
    type Step = S::Frame;

    fn step(&mut self) -> Option<Self::Step> {
        self.0.step()
    }
}

pub struct Phase<X, S, const N: usize>
where
    X: FloatSample,
    S: Step<X, N>,
{
    stepper: S,
    accum: S::Step,
    is_first: bool,
}

impl<X, S, const N: usize> Phase<X, S, N>
where
    X: FloatSample,
    S: Step<X, N>,
{
    pub fn gen_wave<W: WaveFunc<X>>(self, wave_func: W) -> WaveGen<W, S, X, N> {
        WaveGen {
            wave_func,
            phase: self,
        }
    }
}

impl<X, S, const N: usize> From<S> for Phase<X, S, N>
where
    X: FloatSample,
    S: Step<X, N>,
{
    fn from(stepper: S) -> Self {
        Self {
            stepper,
            accum: Frame::EQUILIBRIUM,
            is_first: true,
        }
    }
}

impl<X, S, const N: usize> Signal<N> for Phase<X, S, N>
where
    X: FloatSample,
    S: Step<X, N>,
{
    type Frame = S::Step;

    fn next(&mut self) -> Option<Self::Frame> {
        if self.is_first {
            self.is_first = false;
        } else {
            self.accum = self
                .accum
                .add_frame(self.stepper.step()?.into_signed_frame())
                .map(|x| x % X::one());
        }

        Some(self.accum)
    }
}

impl<X, F, const N: usize> Phase<X, Fixed<F, N>, N>
where
    X: FloatSample,
    F: Frame<N, Sample = X>,
{
    /// Creates a [`Phase`] with a constant [`Frame`] of frequencies.
    ///
    /// This [`Phase`] does not terminate, it will always return a step value.
    ///
    /// ```
    /// use sampara::Signal;
    /// use sampara::wavegen::Phase;
    ///
    /// fn main() {
    ///     let mut phase = Phase::fixed_hz(4.0, [0.5, 1.0, 1.5]);
    ///
    ///     assert_eq!(phase.next(), Some([0.000, 0.000, 0.000]));
    ///     assert_eq!(phase.next(), Some([0.125, 0.250, 0.375]));
    ///     assert_eq!(phase.next(), Some([0.250, 0.500, 0.750]));
    ///     assert_eq!(phase.next(), Some([0.375, 0.750, 0.125]));
    /// }
    /// ```
    pub fn fixed_hz(rate: X, hz: F) -> Self {
        Fixed(hz.map(|x| x / rate)).into()
    }

    /// Creates a [`Phase`] with a constant [`Frame`] of time steps.
    ///
    /// This [`Phase`] does not terminate, it will always return a step value.
    ///
    /// ```
    /// use sampara::Signal;
    /// use sampara::wavegen::Phase;
    ///
    /// fn main() {
    ///     let mut phase = Phase::fixed_step([0.125, 0.250, 0.375]);
    ///
    ///     assert_eq!(phase.next(), Some([0.000, 0.000, 0.000]));
    ///     assert_eq!(phase.next(), Some([0.125, 0.250, 0.375]));
    ///     assert_eq!(phase.next(), Some([0.250, 0.500, 0.750]));
    ///     assert_eq!(phase.next(), Some([0.375, 0.750, 0.125]));
    /// }
    /// ```
    pub fn fixed_step(step: F) -> Self {
        Fixed(step).into()
    }
}

impl<X, S, const N: usize> Phase<X, Variable<S, N>, N>
where
    X: FloatSample,
    S: Signal<N>,
    S::Frame: Frame<N, Sample = X>,
{
    /// Creates a [`Phase`] with [`Frame`]s of frequencies over time, as
    /// yielded by a [`Signal`].
    ///
    /// Unlike [`Phase::fixed_hz`], this [`Phase`] will terminate and stop
    /// yielding step values once the contained [`Signal`] is fully consumed.
    /// If the contained [`Signal`] yields `N` values, then this [`Phase`] will
    /// yield `N + 1` values.
    ///
    /// ```
    /// use sampara::{signal, Signal};
    /// use sampara::wavegen::Phase;
    ///
    /// fn main() {
    ///     let freq_signal = signal::from_frames(vec![
    ///         [0.125, 0.250],
    ///         [0.375, 0.500],
    ///         [0.625, 0.750],
    ///     ]);
    ///
    ///     let mut phase = Phase::variable_hz(4.0, freq_signal);
    ///
    ///     assert_eq!(phase.next(), Some([0.00000, 0.0000]));
    ///     assert_eq!(phase.next(), Some([0.03125, 0.0625]));
    ///     assert_eq!(phase.next(), Some([0.12500, 0.1875]));
    ///     assert_eq!(phase.next(), Some([0.28125, 0.3750]));
    ///     assert_eq!(phase.next(), None);
    /// }
    /// ```
    pub fn variable_hz(rate: X, hz_signal: S) -> Self {
        Variable(VarInner::Hzs(hz_signal, rate)).into()
    }

    /// Creates a [`Phase`] with [`Frame`]s of time steps over time, as
    /// yielded by a [`Signal`].
    ///
    /// Unlike [`Phase::fixed_step`], this [`Phase`] will terminate and stop
    /// yielding step values once the contained [`Signal`] is fully consumed.
    /// If the contained [`Signal`] yields `N` values, then this [`Phase`] will
    /// yield `N + 1` values.
    ///
    /// ```
    /// use sampara::{signal, Signal};
    /// use sampara::wavegen::Phase;
    ///
    /// fn main() {
    ///     let step_signal = signal::from_frames(vec![
    ///         [0.03125, 0.06250],
    ///         [0.37500, 0.50000],
    ///         [0.62500, 0.75000],
    ///     ]);
    ///
    ///     let mut phase = Phase::variable_step(step_signal);
    ///
    ///     assert_eq!(phase.next(), Some([0.00000, 0.0000]));
    ///     assert_eq!(phase.next(), Some([0.03125, 0.0625]));
    ///     assert_eq!(phase.next(), Some([0.40625, 0.5625]));
    ///     assert_eq!(phase.next(), Some([0.03125, 0.3125]));
    ///     assert_eq!(phase.next(), None);
    /// }
    /// ```
    pub fn variable_step(step_signal: S) -> Self {
        Variable(VarInner::Steps(step_signal)).into()
    }
}

pub trait WaveFunc<X>
where
    X: FloatSample,
{
    fn calculate(&self, x_phase: X) -> X;

    fn with_phase<S, const N: usize>(self, phase: Phase<X, S, N>) -> WaveGen<Self, S, X, N>
    where
        Self: Sized,
        S: Step<X, N>,
    {
        phase.gen_wave(self)
    }

    fn fixed_hz<F, const N: usize>(self, rate: X, hz: F) -> WaveGen<Self, Fixed<F, N>, X, N>
    where
        Self: Sized,
        F: Frame<N, Sample = X>,
    {
        self.with_phase(Phase::fixed_hz(rate, hz))
    }

    fn fixed_step<F, const N: usize>(self, step: F) -> WaveGen<Self, Fixed<F, N>, X, N>
    where
        Self: Sized,
        F: Frame<N, Sample = X>,
    {
        self.with_phase(Phase::fixed_step(step))
    }

    fn variable_hz<S, const N: usize>(
        self,
        rate: X,
        hz_signal: S,
    ) -> WaveGen<Self, Variable<S, N>, X, N>
    where
        Self: Sized,
        S: Signal<N>,
        S::Frame: Frame<N, Sample = X>,
    {
        self.with_phase(Phase::variable_hz(rate, hz_signal))
    }

    fn variable_step<S, const N: usize>(self, step_signal: S) -> WaveGen<Self, Variable<S, N>, X, N>
    where
        Self: Sized,
        S: Signal<N>,
        S::Frame: Frame<N, Sample = X>,
    {
        self.with_phase(Phase::variable_step(step_signal))
    }
}

impl<M, X> WaveFunc<X> for M
where
    X: FloatSample,
    M: Fn(X) -> X,
{
    fn calculate(&self, x_phase: X) -> X {
        self(x_phase)
    }
}

/// A sine wave function.
///
/// ```
/// use std::f32::consts::PI;
///
/// use sampara::Signal;
/// use sampara::wavegen::{WaveFunc, Phase, Sine};
///
/// fn main() {
///     const STEP: f32 = 440.0 / 44100.0;
///
///     let mut gen = Phase::fixed_step(STEP).gen_wave(Sine);
///     let (mut x, mut y) = (0.0, 0.0);
///
///     for _ in 0..1000 {
///         y = (2.0 * PI * x).sin();
///         x = (x + STEP) % 1.0;
///
///         assert_eq!(gen.next(), Some(y));
///     }
/// }
/// ```
pub struct Sine;

impl<X> WaveFunc<X> for Sine
where
    X: FloatSample,
{
    fn calculate(&self, x_phase: X) -> X {
        (X::TAU() * x_phase).sin()
    }
}

/// A saw wave function.
///
/// ```
/// use std::f32::consts::PI;
///
/// use sampara::Signal;
/// use sampara::wavegen::{WaveFunc, Phase, Saw};
///
/// fn main() {
///     const STEP: f32 = 440.0 / 44100.0;
///
///     let mut gen = Phase::fixed_step(STEP).gen_wave(Saw);
///     let (mut x, mut y) = (0.0, 0.0);
///
///     for _ in 0..1000 {
///         y = -2.0 * x + 1.0;
///         x = (x + STEP) % 1.0;
///
///         assert_eq!(gen.next(), Some(y));
///     }
/// }
/// ```
pub struct Saw;

impl<X> WaveFunc<X> for Saw
where
    X: FloatSample,
{
    fn calculate(&self, x_phase: X) -> X {
        -(x_phase + x_phase) + X::one()
    }
}

/// A square wave function.
///
/// ```
/// use std::f32::consts::PI;
///
/// use sampara::Signal;
/// use sampara::wavegen::{WaveFunc, Phase, Square};
///
/// fn main() {
///     const STEP: f32 = 440.0 / 44100.0;
///
///     let mut gen = Phase::fixed_step(STEP).gen_wave(Square);
///     let (mut x, mut y) = (0.0, 0.0);
///
///     for _ in 0..1000 {
///         y = if x < 0.5 { 1.0 } else { -1.0 };
///         x = (x + STEP) % 1.0;
///
///         assert_eq!(gen.next(), Some(y));
///     }
/// }
/// ```
pub struct Square;

impl<X> WaveFunc<X> for Square
where
    X: FloatSample,
{
    fn calculate(&self, x_phase: X) -> X {
        if x_phase < X::from(0.5).unwrap() {
            X::one()
        } else {
            -X::one()
        }
    }
}

/// A pulse wave (aka pulse train) function.
///
/// ```
/// use std::f32::consts::PI;
///
/// use sampara::Signal;
/// use sampara::wavegen::{WaveFunc, Phase, Pulse};
///
/// fn main() {
///     const STEP: f32 = 440.0 / 44100.0;
///     const DUTY: f32 = 0.65;
///
///     let mut gen = Phase::fixed_step(STEP).gen_wave(Pulse(DUTY));
///     let (mut x, mut y) = (0.0, 0.0);
///
///     for _ in 0..1000 {
///         y = if x < DUTY { 1.0 } else { -1.0 };
///         x = (x + STEP) % 1.0;
///
///         assert_eq!(gen.next(), Some(y));
///     }
/// }
/// ```
pub struct Pulse<X: FloatSample>(pub X);

impl<X> WaveFunc<X> for Pulse<X>
where
    X: FloatSample,
{
    fn calculate(&self, x_phase: X) -> X {
        if x_phase < self.0 {
            X::one()
        } else {
            -X::one()
        }
    }
}

pub struct WaveGen<W, S, X, const N: usize>
where
    W: WaveFunc<X>,
    X: FloatSample,
    S: Step<X, N>,
{
    wave_func: W,
    phase: Phase<X, S, N>,
}

impl<W, S, X, const N: usize> Signal<N> for WaveGen<W, S, X, N>
where
    W: WaveFunc<X>,
    X: FloatSample,
    S: Step<X, N>,
{
    type Frame = S::Step;

    fn next(&mut self) -> Option<Self::Frame> {
        self.phase
            .next()
            .map(|x_phases| x_phases.map(|x_phase| self.wave_func.calculate(x_phase)))
    }
}
