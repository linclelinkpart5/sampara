use crate::Frame;

pub trait Combinator<const NL: usize, const NR: usize, const NO: usize> {
    type InputL: Frame<NL>;
    type InputR: Frame<NR>;
    type Output: Frame<NO>;

    fn combine(&mut self, input_l: Self::InputL, input_r: Self::InputR) -> Self::Output;
}

/// A [`Combinator`] that calls a closure for each pair of input [`Frame`]s and
/// returns the output.
///
/// ```
/// use sampara::Combinator;
/// use sampara::components::combinators::Mix;
///
/// fn main() {
///     let func = |x: [i8; 2], y: [i8; 2]| {
///         [(x[0] + x[1]) / 2, (y[0] + y[1]) / 2]
///     };
///
///     let mut c = Mix::new(func);
///
///     assert_eq!(c.combine([10, 20], [20, 30]), [15, 25]);
///     assert_eq!(c.combine([-5, 25], [-5, 25]), [10, 10]);
///     assert_eq!(c.combine([30, -20], [40, -30]), [5, 5]);
/// }
/// ```
pub struct Mix<FL, FR, FO, M, const NL: usize, const NR: usize, const NO: usize>
where
    FL: Frame<NL>,
    FR: Frame<NR>,
    FO: Frame<NO>,
    M: FnMut(FL, FR) -> FO,
{
    pub(super) func: M,
    pub(super) _marker: std::marker::PhantomData<(FL, FR, FO)>,
}

impl<FL, FR, FO, M, const NL: usize, const NR: usize, const NO: usize>
    Mix<FL, FR, FO, M, NL, NR, NO>
where
    FL: Frame<NL>,
    FR: Frame<NR>,
    FO: Frame<NO>,
    M: FnMut(FL, FR) -> FO,
{
    pub fn new(func: M) -> Self {
        Self {
            func,
            _marker: Default::default(),
        }
    }
}

impl<FL, FR, FO, M, const NL: usize, const NR: usize, const NO: usize> Combinator<NL, NR, NO>
for Mix<FL, FR, FO, M, NL, NR, NO>
where
    FL: Frame<NL>,
    FR: Frame<NR>,
    FO: Frame<NO>,
    M: FnMut(FL, FR) -> FO,
{
    type InputL = FL;
    type InputR = FR;
    type Output = FO;

    fn combine(&mut self, input_l: Self::InputL, input_r: Self::InputR) -> Self::Output {
        (self.func)(input_l, input_r)
    }
}

impl<FL, FR, FO, M, const NL: usize, const NR: usize, const NO: usize> From<M>
for Mix<FL, FR, FO, M, NL, NR, NO>
where
    FL: Frame<NL>,
    FR: Frame<NR>,
    FO: Frame<NO>,
    M: FnMut(FL, FR) -> FO,
{
    fn from(func: M) -> Self {
        Self::new(func)
    }
}
