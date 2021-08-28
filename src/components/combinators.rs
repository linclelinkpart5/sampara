use std::marker::PhantomData;

pub trait Combinator {
    type InputL;
    type InputR;
    type Output;

    fn combine(&mut self, input_l: Self::InputL, input_r: Self::InputR) -> Self::Output;
}

pub trait StatefulCombinator {
    type InputL;
    type InputR;
    type Output;

    fn advance(&mut self, input_l: Self::InputL, input_r: Self::InputR);
    fn current(&self) -> Self::Output;
}

impl<S> Combinator for S
where
    S: StatefulCombinator,
{
    type InputL = S::InputL;
    type InputR = S::InputR;
    type Output = S::Output;

    fn combine(&mut self, input_l: Self::InputL, input_r: Self::InputR) -> Self::Output {
        self.advance(input_l, input_r);
        self.current()
    }
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
pub struct Mix<L, R, O, M>
where
    M: FnMut(L, R) -> O,
{
    func: M,
    _marker: PhantomData<(L, R, O)>,
}

impl<L, R, O, M> Mix<L, R, O, M>
where
    M: FnMut(L, R) -> O,
{
    pub fn new(func: M) -> Self {
        Self {
            func,
            _marker: Default::default(),
        }
    }
}

impl<L, R, O, M> Combinator for Mix<L, R, O, M>
where
    M: FnMut(L, R) -> O,
{
    type InputL = L;
    type InputR = R;
    type Output = O;

    fn combine(&mut self, input_l: Self::InputL, input_r: Self::InputR) -> Self::Output {
        (self.func)(input_l, input_r)
    }
}

impl<L, R, O, M> From<M> for Mix<L, R, O, M>
where
    M: FnMut(L, R) -> O,
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
pub struct Selector<T> {
    select_right: bool,
    _marker: PhantomData<T>,
}

impl<T> Selector<T> {
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

impl<T> Combinator for Selector<T> {
    type InputL = T;
    type InputR = T;
    type Output = T;

    fn combine(&mut self, input_l: Self::InputL, input_r: Self::InputR) -> Self::Output {
        if self.select_right {
            input_r
        } else {
            input_l
        }
    }
}
