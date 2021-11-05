use std::marker::PhantomData;

pub trait Generator {
    type Output;

    fn generate(&mut self) -> Self::Output;
}

pub trait StatefulGenerator {
    type Output;

    fn advance(&mut self);
    fn current(&self) -> Self::Output;
}

impl<S> Generator for S
where
    S: StatefulGenerator,
{
    type Output = S::Output;

    fn generate(&mut self) -> Self::Output {
        self.advance();
        self.current()
    }
}

/// A [`Generator`] that calls a closure to produce each output [`Frame`].
///
/// ```
/// use sampara::Generator;
/// use sampara::components::generators::GenFn;
///
/// fn main() {
///     let mut i = 0;
///     let mut gen = GenFn::new(|| {
///         i += 1;
///         [i * 10, i * 20]
///     });
///
///     assert_eq!(gen.generate(), [10, 20]);
///     assert_eq!(gen.generate(), [20, 40]);
///     assert_eq!(gen.generate(), [30, 60]);
/// }
/// ```
pub struct GenFn<T, M>
where
    M: FnMut() -> T,
{
    func: M,
    _marker: PhantomData<T>,
}

impl<T, M> GenFn<T, M>
where
    M: FnMut() -> T,
{
    pub fn new(func: M) -> Self {
        Self {
            func,
            _marker: Default::default(),
        }
    }
}

impl<T, M> Generator for GenFn<T, M>
where
    M: FnMut() -> T,
{
    type Output = T;

    fn generate(&mut self) -> Self::Output {
        (self.func)()
    }
}
