use core::f64::consts::PI;

use crate::{Frame, Signal};

enum VarPhaseInner<S, const N: usize>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    Norm(S, f64),
    Raw(S),
}

impl<S, const N: usize> Signal<N> for VarPhaseInner<S, N>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    type Frame = S::Frame;

    #[inline]
    fn next(&mut self) -> Option<Self::Frame> {
        match self {
            Self::Norm(signal, rate) => {
                signal.next().map(|f| f.mul_amp(1.0 / *rate))
            },
            Self::Raw(signal) => {
                signal.next()
            },
        }
    }
}
