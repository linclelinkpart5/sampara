use crate::Frame;

pub trait Processor<const NI: usize, const NO: usize> {
    type Input: Frame<NI>;
    type Output: Frame<NO>;

    fn process(&mut self, input: Self::Input) -> Self::Output;
}

pub trait BlockingProcessor<const NI: usize, const NO: usize> {
    type Input: Frame<NI>;
    type Output: Frame<NO>;

    fn try_process(&mut self, input: Self::Input) -> Option<Self::Output>;
}

impl<P, const NI: usize, const NO: usize> BlockingProcessor<NI, NO> for P
where
    P: Processor<NI, NO>,
{
    type Input = P::Input;
    type Output = P::Output;

    fn try_process(&mut self, input: Self::Input) -> Option<Self::Output> {
        Some(self.process(input))
    }
}

/// A [`Processor`] that calls a closure for each input [`Frame`] and returns
/// the output.
///
/// ```
/// use sampara::Processor;
/// use sampara::components::processors::Map;
///
/// fn main() {
///     let mut i = 0;
///     let func = |x: [i8; 2]| { i += 10; [x[0] + i, x[1] - i] };
///
///     let mut p = Map::new(func);
///
///     assert_eq!(p.process([0, 0]), [10, -10]);
///     assert_eq!(p.process([0, 0]), [20, -20]);
///     assert_eq!(p.process([-30, 30]), [0, 0]);
/// }
/// ```
pub struct Map<FI, FO, M, const NI: usize, const NO: usize>
where
    FI: Frame<NI>,
    FO: Frame<NO>,
    M: FnMut(FI) -> FO,
{
    pub(super) func: M,
    pub(super) _marker: std::marker::PhantomData<(FI, FO)>,
}

impl<FI, FO, M, const NI: usize, const NO: usize> Map<FI, FO, M, NI, NO>
where
    FI: Frame<NI>,
    FO: Frame<NO>,
    M: FnMut(FI) -> FO,
{
    pub fn new(func: M) -> Self {
        Self {
            func,
            _marker: Default::default(),
        }
    }
}

impl<FI, FO, M, const NI: usize, const NO: usize> Processor<NI, NO> for Map<FI, FO, M, NI, NO>
where
    FI: Frame<NI>,
    FO: Frame<NO>,
    M: FnMut(FI) -> FO,
{
    type Input = FI;
    type Output = FO;

    fn process(&mut self, input: Self::Input) -> Self::Output {
        (self.func)(input)
    }
}

impl<M, FI, FO, const NI: usize, const NO: usize> From<M> for Map<FI, FO, M, NI, NO>
where
    FI: Frame<NI>,
    FO: Frame<NO>,
    M: FnMut(FI) -> FO,
{
    fn from(func: M) -> Self {
        Self::new(func)
    }
}

/// A [`Processor`] that feeds its input to an inner [`Processor`], and then
/// feeds that output into another inner [`Processor`], like a chain.
///
/// ```
/// use sampara::Processor;
/// use sampara::components::processors::{Chain, Map};
///
/// fn main() {
///     let mut pa = Map::new(|x| x + 1);
///     let mut pb = Map::new(|x| x * 2);
///     let mut p = Chain::new(pa, pb);
///
///     assert_eq!(p.process(0), 2);
///     assert_eq!(p.process(3), 8);
///     assert_eq!(p.process(-3), -4);
/// }
/// ```
pub struct Chain<PA, PB, const NI: usize, const NX: usize, const NO: usize>
where
    PA: Processor<NI, NX>,
    PB: Processor<NX, NO, Input = PA::Output>,
{
    pub(super) processor_a: PA,
    pub(super) processor_b: PB,
}

impl<PA, PB, const NI: usize, const NX: usize, const NO: usize> Chain<PA, PB, NI, NX, NO>
where
    PA: Processor<NI, NX>,
    PB: Processor<NX, NO, Input = PA::Output>,
{
    pub fn new(processor_a: PA, processor_b: PB) -> Self {
        Self {
            processor_a,
            processor_b,
        }
    }
}

impl<PA, PB, const NI: usize, const NX: usize, const NO: usize> Processor<NI, NO>
for Chain<PA, PB, NI, NX, NO>
where
    PA: Processor<NI, NX>,
    PB: Processor<NX, NO, Input = PA::Output>,
{
    type Input = PA::Input;
    type Output = PB::Output;

    fn process(&mut self, input: Self::Input) -> Self::Output {
        let inter = self.processor_a.process(input);
        self.processor_b.process(inter)
    }
}
