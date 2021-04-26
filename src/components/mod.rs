use crate::Frame;

pub trait Processor<const NI: usize, const NO: usize> {
    type Input: Frame<NI>;
    type Output: Frame<NO>;

    fn process(&mut self, input: Self::Input) -> Self::Output;
}

pub trait Generator<const NO: usize> {
    type Output: Frame<NO>;

    fn generate(&mut self) -> Self::Output;
}

pub trait Combinator<const NA: usize, const NB: usize, const NO: usize> {
    type InputA: Frame<NA>;
    type InputB: Frame<NB>;
    type Output: Frame<NO>;

    fn combine(&mut self, input_a: Self::InputA, input_b: Self::InputB) -> Self::Output;
}

pub trait Splitter<const NI: usize, const NA: usize, const NB: usize> {
    type Input: Frame<NI>;
    type OutputA: Frame<NA>;
    type OutputB: Frame<NB>;

    fn split(&mut self, input: Self::Input) -> (Self::OutputA, Self::OutputB);
}
