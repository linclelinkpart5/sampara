use crate::{Frame, Signal};
use crate::sample::FloatSample;

pub trait Step<F, const N: usize>
where
    F: Frame<N>,
    F::Sample: FloatSample,
{
    fn step(&mut self) -> Option<F>;
}

pub struct Fixed<F, const N: usize>(F)
where
    F: Frame<N>,
    F::Sample: FloatSample;

impl<F, const N: usize> Step<F, N> for Fixed<F, N>
where
    F: Frame<N>,
    F::Sample: FloatSample,
{
    fn step(&mut self) -> Option<F> {
        Some(self.0)
    }
}

enum VarInner<S, const N: usize>
where
    S: Signal<N>,
    <S::Frame as Frame<N>>::Sample: FloatSample,
{
    Hzs(S, <S::Frame as Frame<N>>::Sample),
    Deltas(S),
}

impl<S, const N: usize> Step<S::Frame, N> for VarInner<S, N>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    fn step(&mut self) -> Option<S::Frame> {
        match self {
            Self::Hzs(hz_signal, rate) => {
                hz_signal.next().map(|f| f.mul_amp(1.0 / *rate))
            },
            Self::Deltas(delta_signal) => {
                delta_signal.next()
            },
        }
    }
}

pub struct Variable<S, const N: usize>(VarInner<S, N>)
where
    S: Signal<N>,
    <S::Frame as Frame<N>>::Sample: FloatSample;

impl<S, const N: usize> Step<S::Frame, N> for Variable<S, N>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    fn step(&mut self) -> Option<S::Frame> {
        self.0.step()
    }
}
