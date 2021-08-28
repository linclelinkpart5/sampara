pub trait Calculator {
    type Input;
    type Output;

    fn push(&mut self, input: Self::Input);
    fn calculate(self) -> Self::Output;
}
