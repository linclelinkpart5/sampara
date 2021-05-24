pub mod processors;
pub mod combinators;

pub use processors::Processor;
pub use combinators::Combinator;

use crate::Frame;

pub trait Generator<const NO: usize> {
    type Output: Frame<NO>;

    fn generate(&mut self) -> Self::Output;
}

pub trait Splitter<const NI: usize, const NA: usize, const NB: usize> {
    type Input: Frame<NI>;
    type OutputA: Frame<NA>;
    type OutputB: Frame<NB>;

    fn split(&mut self, input: Self::Input) -> (Self::OutputA, Self::OutputB);
}
