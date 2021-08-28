pub trait Processor {
    type Input;
    type Output;

    fn process(&mut self, input: Self::Input) -> Self::Output;
}

pub trait StatefulProcessor {
    type Input;
    type Output;

    fn advance(&mut self, input: Self::Input);
    fn current(&self) -> Self::Output;
}

impl<S> Processor for S
where
    S: StatefulProcessor,
{
    type Input = S::Input;
    type Output = S::Output;

    fn process(&mut self, input: Self::Input) -> Self::Output {
        self.advance(input);
        self.current()
    }
}

/// A [`Processor`] that calls a closure to map each input to an output.
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
pub struct Map<I, O, M>
where
    M: FnMut(I) -> O,
{
    pub(super) func: M,
    pub(super) _marker: std::marker::PhantomData<(I, O)>,
}

impl<I, O, M> Map<I, O, M>
where
    M: FnMut(I) -> O,
{
    pub fn new(func: M) -> Self {
        Self {
            func,
            _marker: Default::default(),
        }
    }
}

impl<I, O, M> Processor for Map<I, O, M>
where
    M: FnMut(I) -> O,
{
    type Input = I;
    type Output = O;

    fn process(&mut self, input: Self::Input) -> Self::Output {
        (self.func)(input)
    }
}

impl<M, I, O> From<M> for Map<I, O, M>
where
    M: FnMut(I) -> O,
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
pub struct Chain<PA, PB>
where
    PA: Processor,
    PB: Processor<Input = PA::Output>,
{
    pub(super) processor_a: PA,
    pub(super) processor_b: PB,
}

impl<PA, PB> Chain<PA, PB>
where
    PA: Processor,
    PB: Processor<Input = PA::Output>,
{
    pub fn new(processor_a: PA, processor_b: PB) -> Self {
        Self {
            processor_a,
            processor_b,
        }
    }
}

impl<PA, PB> Processor for Chain<PA, PB>
where
    PA: Processor,
    PB: Processor<Input = PA::Output>,
{
    type Input = PA::Input;
    type Output = PB::Output;

    fn process(&mut self, input: Self::Input) -> Self::Output {
        let inter = self.processor_a.process(input);
        self.processor_b.process(inter)
    }
}
