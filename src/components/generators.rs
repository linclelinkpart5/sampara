use std::marker::PhantomData;

use crate::Frame;

pub trait Generator<const NO: usize> {
    type Output: Frame<NO>;

    fn generate(&mut self) -> Self::Output;
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
pub struct GenFn<FO, M, const NO: usize>
where
    FO: Frame<NO>,
    M: FnMut() -> FO,
{
    func: M,
    _marker: PhantomData<FO>,
}

impl<FO, M, const NO: usize> GenFn<FO, M, NO>
where
    FO: Frame<NO>,
    M: FnMut() -> FO,
{
    pub fn new(func: M) -> Self {
        Self {
            func,
            _marker: Default::default(),
        }
    }
}

impl<FO, M, const NO: usize> Generator<NO> for GenFn<FO, M, NO>
where
    FO: Frame<NO>,
    M: FnMut() -> FO,
{
    type Output = FO;

    fn generate(&mut self) -> Self::Output {
        (self.func)()
    }
}

