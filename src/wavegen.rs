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

pub struct Phase<X, S, const N: usize>
where
    X: FloatSample,
    S: Step<X, N>,
{
    stepper: S,
    accum: S::Step,
}

impl<X, S, const N: usize> From<S> for Phase<X, S, N>
where
    X: FloatSample,
    S: Step<X, N>,
{
    fn from(stepper: S) -> Self {
        Self { stepper, accum: Frame::EQUILIBRIUM }
    }
}

impl<X, S, const N: usize> Signal<N> for Phase<X, S, N>
where
    X: FloatSample,
    S: Step<X, N>,
{
    type Frame = S::Step;

    fn next(&mut self) -> Option<Self::Frame> {
        let phase = self.accum
            .add_frame(self.stepper.step()?.into_signed_frame())
            .apply(|x| x % X::one());

        self.accum = phase;
        Some(phase)
    }
}

pub fn fixed_hz<X, F, const N: usize>(rate: X, hz: F) -> Phase<X, Fixed<F, N>, N>
where
    X: FloatSample,
    F: Frame<N, Sample = X>,
{
    Fixed(hz.apply(|x| x / rate)).into()
}

pub fn fixed_step<X, F, const N: usize>(step: F) -> Phase<X, Fixed<F, N>, N>
where
    X: FloatSample,
    F: Frame<N, Sample = X>,
{
    Fixed(step).into()
}

pub fn variable_hz<X, S, const N: usize>(rate: X, hz_signal: S) -> Phase<X, Variable<S, N>, N>
where
    X: FloatSample,
    S: Signal<N>,
    S::Frame: Frame<N, Sample = X>,
{
    Variable(VarInner::Hzs(hz_signal, rate)).into()
}

pub fn variable_step<X, S, const N: usize>(delta_signal: S) -> Phase<X, Variable<S, N>, N>
where
    X: FloatSample,
    S: Signal<N>,
    S::Frame: Frame<N, Sample = X>,
{
    Variable(VarInner::Deltas(delta_signal)).into()
}
