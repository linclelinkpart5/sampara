use crate::{Frame, Signal};

enum VarStepInner<S, const N: usize>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    Hzs(S, f64),
    Steps(S),
}

impl<S, const N: usize> Signal<N> for VarStepInner<S, N>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    type Frame = S::Frame;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        match self {
            Self::Hzs(hz_signal, rate) => {
                hz_signal.next().map(|f| f.mul_amp(1.0 / *rate))
            },
            Self::Steps(step_signal) => {
                step_signal.next()
            },
        }
    }
}

pub struct FixedStep<F, const N: usize>
where
    F: Frame<N, Sample = f64>,
{
    step: F,
}

pub struct VariableStep<S, const N: usize>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    inner: VarStepInner<S, N>,
}

pub fn fixed_hz<F, const N: usize>(rate: f64, hz: F) -> FixedStep<F, N>
where
    F: Frame<N, Sample = f64>,
{
    let step: F = hz.apply(|x| x / rate);

    FixedStep { step }
}

pub fn variable_hz<S, const N: usize>(rate: f64, hzs: S) -> VariableStep<S, N>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    VariableStep { inner: VarStepInner::Hzs(hzs, rate) }
}

pub fn fixed_step<F, const N: usize>(step: F) -> FixedStep<F, N>
where
    F: Frame<N, Sample = f64>,
{
    FixedStep { step }
}

pub fn variable_step<S, const N: usize>(steps: S) -> VariableStep<S, N>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    VariableStep { inner: VarStepInner::Steps(steps) }
}
