use std::marker::PhantomData;

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
    func: M,
    _marker: PhantomData<(FL, FR, FO)>,
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

/// A [`Combinator`] that returns one of the two input [`Frame`]s as determined
/// by a selector state. This selector state can be changed at runtime in order
/// to switch between returning the left/right input [`Frame`].
///
/// ```
/// use sampara::Combinator;
/// use sampara::components::combinators::Selector;
///
/// fn main() {
///     const L: [u8; 3] = [1, 2, 3];
///     const R: [u8; 3] = [4, 5, 6];
///
///     let mut c = Selector::left();
///     assert_eq!(c.combine(L, R), L);
///
///     c.set_right();
///     assert_eq!(c.combine(L, R), R);
///
///     c.toggle();
///     assert_eq!(c.combine(L, R), L);
/// }
/// ```
pub struct Selector<F, const N: usize>
where
    F: Frame<N>,
{
    select_right: bool,
    _marker: PhantomData<F>,
}

impl<F, const N: usize> Selector<F, N>
where
    F: Frame<N>,
{
    fn new(r: bool) -> Self {
        Self {
            select_right: r,
            _marker: Default::default(),
        }
    }

    pub fn left() -> Self {
        Self::new(false)
    }

    pub fn right() -> Self {
        Self::new(true)
    }

    pub fn set_left(&mut self) {
        self.select_right = false
    }

    pub fn set_right(&mut self) {
        self.select_right = true
    }

    pub fn toggle(&mut self) {
        self.select_right = !self.select_right
    }
}

impl<F, const N: usize> Combinator<N, N, N> for Selector<F, N>
where
    F: Frame<N>,
{
    type InputL = F;
    type InputR = F;
    type Output = F;

    fn combine(&mut self, input_l: Self::InputL, input_r: Self::InputR) -> Self::Output {
        if self.select_right {
            input_r
        } else {
            input_l
        }
    }
}
