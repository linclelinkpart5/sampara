use crate::Frame;

pub trait Calculator<const N: usize> {
    type Input: Frame<N>;
    type Output;

    fn push(&mut self, input: Self::Input);
    fn calculate(self) -> Self::Output;
}
