use core::f64::consts::PI;

use crate::{Frame, Signal};

/// Types that can produce a phase step size, usually based on a target
/// frequency divided by a sampling frequency (sample rate).
///
/// These types are mainly used for driving oscillators and other periodic
/// [`Signal`]s, which advance one step at a time for each output.
pub trait Step<const N: usize> {
    type Step: Frame<N, Sample = f64>;

    fn step(&mut self) -> Option<Self::Step>;
}

pub struct ConstHz<F, const N: usize>
where
    F: Frame<N, Sample = f64>,
{
    step: F,
}

impl<F, const N: usize> ConstHz<F, N>
where
    F: Frame<N, Sample = f64>,
{
    pub fn new(rate: f64, hz: F) -> Self {
        let step = hz.apply(|x| x / rate);
        Self { step }
    }
}

pub struct VariableHz<S, const N: usize>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    hzs: S,
    rate: f64,
}

impl<S, const N: usize> VariableHz<S, N>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    pub fn new(rate: f64, hz_signal: S) -> Self {
        Self {
            hzs: hz_signal,
            rate,
        }
    }
}

impl<F, const N: usize> Step<N> for ConstHz<F, N>
where
    F: Frame<N, Sample = f64>,
{
    type Step = F;

    fn step(&mut self) -> Option<Self::Step> {
        Some(self.step)
    }
}

impl<S, const N: usize> Step<N> for VariableHz<S, N>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    type Step = S::Frame;

    fn step(&mut self) -> Option<Self::Step> {
        self.hzs.next().map(|f| f.mul_amp(1.0 / self.rate))
    }
}

/// A [`Signal`] that wraps a [`Step`] and accumulates it in an automated way,
/// wrapping it to the interval [0.0, 1.0) as needed.
pub struct Phase<S, const N: usize>
where
    S: Step<N>,
{
    stepper: S,
    accum: S::Step,
}

impl<S, const N: usize> Signal<N> for Phase<S, N>
where
    S: Step<N>,
{
    type Frame = S::Step;

    fn next(&mut self) -> Option<Self::Frame> {
        let phase = self.accum
            .add_frame(self.stepper.step()?.into_signed_frame())
            .apply(|x| x % 1.0);

        self.accum = phase;
        Some(phase)
    }
}

/// A sine wave [`Signal`] generator.
pub struct Sine<S, const N: usize>
where
    S: Step<N>,
{
    phase: Phase<S, N>,
}

impl<S, const N: usize> Signal<N> for Sine<S, N>
where
    S: Step<N>,
{
    type Frame = S::Step;

    fn next(&mut self) -> Option<Self::Frame> {
        self.phase.next().map(|mut phase| {
            phase.transform(|p| (2.0 * PI * p).sin());
            phase
        })
    }
}

/// A saw wave [`Signal`] generator.
pub struct Saw<S, const N: usize>
where
    S: Step<N>,
{
    phase: Phase<S, N>,
}

impl<S, const N: usize> Signal<N> for Saw<S, N>
where
    S: Step<N>,
{
    type Frame = S::Step;

    fn next(&mut self) -> Option<Self::Frame> {
        self.phase.next().map(|mut phase| {
            phase.transform(|p| p * -2.0 + 1.0);
            phase
        })
    }
}

/// A square wave [`Signal`] generator.
pub struct Square<S, const N: usize>
where
    S: Step<N>,
{
    phase: Phase<S, N>,
}

impl<S, const N: usize> Signal<N> for Square<S, N>
where
    S: Step<N>,
{
    type Frame = S::Step;

    fn next(&mut self) -> Option<Self::Frame> {
        self.phase.next().map(|mut phase| {
            phase.transform(|p| {
                if p < 0.5 { 1.0 }
                else { -1.0 }
            });
            phase
        })
    }
}
