use crate::{Frame, Signal};
use crate::sample::FloatSample;

struct SummageInner<F, const N: usize, const SQRT: bool, const POW2: bool>
where
    F: Frame<N>,
    F::Sample: FloatSample,
{
    sum: F,
    count: u64,
}

struct MinMaxInner<F, const N: usize, const MAX: bool>
where
    F: Frame<N>,
{
    frontier: F,
}
