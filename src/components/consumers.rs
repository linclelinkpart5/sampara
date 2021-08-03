use crate::Frame;

pub trait Consumer<const N: usize> {
    type Input: Frame<N>;
    type Output;

    fn push(&mut self, input: Self::Input);
    fn consume(self) -> Self::Output;
}
