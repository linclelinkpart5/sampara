use crate::{Frame, Signal};

/// Types that can produce a phase step size, usually based on a target
/// frequency divided by a sampling frequency (sample rate).
///
/// These types are mainly used for driving oscillators and other periodic
/// [`Signal`]s, which advance one step at a time for each output.
pub trait Step<F, const N: usize>
where
    F: Frame<N, Sample = f64>
{
    fn step(&mut self) -> Option<F>;
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

impl<F, const N: usize> Step<F, N> for ConstHz<F, N>
where
    F: Frame<N, Sample = f64>,
{
    fn step(&mut self) -> Option<F> {
        Some(self.step)
    }
}

impl<S, const N: usize> Step<S::Frame, N> for VariableHz<S, N>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    fn step(&mut self) -> Option<S::Frame> {
        self.hzs.next().map(|f| f.mul_amp(1.0 / self.rate))
    }
}
