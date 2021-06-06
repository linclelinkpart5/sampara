use num_traits::Float;

use crate::{Frame, Signal};
use crate::sample::FloatSample;

// LEARN: Good example of the difference between type generics and associated
//        types.
// pub trait OldStep<F, const N: usize>
// where
//     F: Frame<N>,
//     F::Sample: FloatSample,
// {
//     fn step(&mut self) -> Option<F>;
// }

pub trait Step<S, const N: usize>
where
    S: FloatSample,
{
    type Step: Frame<N, Sample = S>;

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
    Deltas(S),
}

impl<S, const N: usize> Step<<S::Frame as Frame<N>>::Sample, N> for VarInner<S, N>
where
    S: Signal<N>,
    <S::Frame as Frame<N>>::Sample: FloatSample,
{
    type Step = S::Frame;

    fn step(&mut self) -> Option<Self::Step> {
        match self {
            Self::Hzs(hz_signal, rate) => {
                hz_signal.next().map(|f| f.mul_amp(rate.recip()))
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

pub struct Phase<S, X, const N: usize>
where
    S: FloatSample,
    X: Step<S, N>,
{
    stepper: X,
    accum: X::Step,
}
