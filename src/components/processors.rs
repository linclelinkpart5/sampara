use crate::Frame;

pub trait Processor<const NI: usize, const NO: usize> {
    type Input: Frame<NI>;
    type Output: Frame<NO>;

    fn process(&mut self, input: Self::Input) -> Self::Output;
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
