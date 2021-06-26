pub mod types;

pub use types::*;

use std::ops::Range;
use std::option::IntoIter as OptionIntoIter;

use num_traits::Float;

use crate::buffer::Buffer;

const DO_BACK: bool = true;
const NO_BACK: bool = false;

const DO_SYMM: bool = true;
const NO_SYMM: bool = false;

pub trait Window<F: Float> {
    /// Given a value in the interval [0.0, 1.0], returns the value of the
    /// window function at that point.
    fn calc(&self, x: F) -> F;

    /// Returns an iterator that yields the values of a symmetric window of
    /// length `N`.
    ///
    /// The `N` input values for a symmetric window evenly span the input
    /// interval [0.0, 1.0].
    ///
    /// ```
    /// use sampara::window::Window;
    /// use sampara::window::types::Triangle;
    ///
    /// fn main() {
    ///     let mut iter = Window::iter(Triangle, 4);
    ///
    ///     assert_eq!(iter.next(), Some(0.0f64));
    ///     assert_eq!(iter.next(), Some(0.6666666666666666));
    ///     assert_eq!(iter.next(), Some(0.6666666666666667));
    ///     assert_eq!(iter.next(), Some(0.0));
    ///     assert_eq!(iter.next(), None);
    ///
    ///     // One point always yields a single value of `1.0`.
    ///     let mut iter = Window::iter(Triangle, 1);
    ///
    ///     assert_eq!(iter.next(), Some(1.0f64));
    ///     assert_eq!(iter.next(), None);
    ///
    ///     // Zero points is an empty iterator.
    ///     let mut iter = Window::<f64>::iter(Triangle, 0);
    ///
    ///     assert_eq!(iter.next(), None);
    /// }
    /// ```
    fn iter(self, len: usize) -> Iter<Self, F>
    where
        Self: Sized,
    {
        Iter(IterImpl::new(len, self))
    }

    /// Returns an iterator that yields the values of a periodic window of
    /// length `N`.
    ///
    /// A periodic window of length `N` is equivalent to a symmetric window of
    /// length `N + 1` with its last value omitted.
    ///
    /// ```
    /// use sampara::window::Window;
    /// use sampara::window::types::Triangle;
    ///
    /// fn main() {
    ///     let mut iter = Window::iter_periodic(Triangle, 4);
    ///
    ///     // The first 4 values of a symmetric window of length 5.
    ///     assert_eq!(iter.next(), Some(0.0f64));
    ///     assert_eq!(iter.next(), Some(0.5));
    ///     assert_eq!(iter.next(), Some(1.0));
    ///     assert_eq!(iter.next(), Some(0.5));
    ///     assert_eq!(iter.next(), None);
    ///
    ///     // The first value of a symmetric window of length 2.
    ///     let mut iter = Window::iter_periodic(Triangle, 1);
    ///
    ///     assert_eq!(iter.next(), Some(0.0f64));
    ///     assert_eq!(iter.next(), None);
    ///
    ///     // Zero points is an empty iterator.
    ///     let mut iter = Window::<f64>::iter_periodic(Triangle, 0);
    ///
    ///     assert_eq!(iter.next(), None);
    /// }
    /// ```
    fn iter_periodic(self, len: usize) -> IterPeriodic<Self, F>
    where
        Self: Sized,
    {
        IterPeriodic(IterImpl::new(len, self))
    }

    /// Fills a buffer of length `N` with the values of a symmetric window of
    /// length `N`.
    ///
    /// ```
    /// use sampara::window::Window;
    /// use sampara::window::types::Triangle;
    ///
    /// fn main() {
    ///     let mut buffer = [-1.0f64; 4];
    ///     Window::fill(Triangle, &mut buffer);
    ///     assert_eq!(buffer, [0.0, 0.6666666666666666, 0.6666666666666667, 0.0]);
    ///
    ///     let mut buffer = [-1.0f64; 1];
    ///     Window::fill(Triangle, &mut buffer);
    ///     assert_eq!(buffer, [1.0]);
    ///
    ///     let mut buffer = [-1.0f64; 0];
    ///     Window::fill(Triangle, &mut buffer);
    ///     assert_eq!(buffer, []);
    /// }
    /// ```
    fn fill<B>(self, buffer: &mut B)
    where
        Self: Sized,
        B: Buffer<Item = F>,
    {
        let slice = buffer.as_mut();
        let window = self.iter(slice.len());

        for (buf, w) in slice.iter_mut().zip(window) {
            *buf = w;
        }
    }

    /// Fills a buffer of length `N` with the values of a periodic window of
    /// length `N`.
    ///
    /// ```
    /// use sampara::window::Window;
    /// use sampara::window::types::Triangle;
    ///
    /// fn main() {
    ///     let mut buffer = [-1.0f64; 4];
    ///     Window::fill_periodic(Triangle, &mut buffer);
    ///     assert_eq!(buffer, [0.0, 0.5, 1.0, 0.5]);
    ///
    ///     let mut buffer = [-1.0f64; 1];
    ///     Window::fill_periodic(Triangle, &mut buffer);
    ///     assert_eq!(buffer, [0.0]);
    ///
    ///     let mut buffer = [-1.0f64; 0];
    ///     Window::fill_periodic(Triangle, &mut buffer);
    ///     assert_eq!(buffer, []);
    /// }
    /// ```
    fn fill_periodic<B>(self, buffer: &mut B)
    where
        Self: Sized,
        B: Buffer<Item = F>,
    {
        let slice = buffer.as_mut();
        let window = self.iter_periodic(slice.len());

        for (buf, w) in slice.iter_mut().zip(window) {
            *buf = w;
        }
    }

    /// Element-wise multiplies a buffer of length `N` with the values of a
    /// symmetric window of length `N`.
    ///
    /// ```
    /// use sampara::window::Window;
    /// use sampara::window::types::Triangle;
    ///
    /// fn main() {
    ///     let mut buffer = [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];
    ///     Window::apply(Triangle, &mut buffer);
    ///     assert_eq!(buffer, [
    ///         0.0,
    ///         0.022222222222222223,
    ///         0.08888888888888889,
    ///         0.19999999999999998,
    ///         0.35555555555555557,
    ///         0.4444444444444444,
    ///         0.4,
    ///         0.3111111111111112,
    ///         0.17777777777777787,
    ///         0.0,
    ///     ]);
    /// }
    /// ```
    fn apply<B>(self, buffer: &mut B)
    where
        Self: Sized,
        B: Buffer<Item = F>,
    {
        let slice = buffer.as_mut();
        let window = self.iter(slice.len());

        for (buf, w) in slice.iter_mut().zip(window) {
            *buf = *buf * w;
        }
    }

    /// Element-wise multiplies a buffer of length `N` with the values of a
    /// periodic window of length `N`.
    ///
    /// ```
    /// use sampara::window::Window;
    /// use sampara::window::types::Triangle;
    ///
    /// fn main() {
    ///     let mut buffer = [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];
    ///     Window::apply_periodic(Triangle, &mut buffer);
    ///     assert_eq!(buffer, [
    ///         0.0,
    ///         0.019999999999999997,
    ///         0.08000000000000002,
    ///         0.18000000000000002,
    ///         0.32000000000000006,
    ///         0.5,
    ///         0.47999999999999987,
    ///         0.4199999999999999,
    ///         0.31999999999999995,
    ///         0.17999999999999997,
    ///     ]);
    /// }
    /// ```
    fn apply_periodic<B>(self, buffer: &mut B)
    where
        Self: Sized,
        B: Buffer<Item = F>,
    {
        let slice = buffer.as_mut();
        let window = self.iter_periodic(slice.len());

        for (buf, w) in slice.iter_mut().zip(window) {
            *buf = *buf * w;
        }
    }
}

enum IterImpl<W, F, const SYMM: bool>
where
    W: Window<F>,
    F: Float,
{
    ZeroOne(OptionIntoIter<()>),
    Normal(Range<usize>, F, W),
}

impl<W, F, const SYMM: bool> IterImpl<W, F, SYMM>
where
    W: Window<F>,
    F: Float,
{
    fn new(len: usize, windower: W) -> Self {
        let bins = match len {
            0 => return Self::ZeroOne(None.into_iter()),
            1 if SYMM => return Self::ZeroOne(Some(()).into_iter()),
            n if SYMM => n - 1,
            n => n,
        };

        let factor = F::from(bins).unwrap().recip();
        Self::Normal(0..len, factor, windower)
    }

    #[inline]
    fn advance<const BACK: bool>(&mut self) -> Option<<Self as Iterator>::Item> {
        match self {
            Self::ZeroOne(it) => {
                let opt = if BACK {
                    it.next_back()
                }
                else {
                    it.next()
                };

                opt.map(|_| F::one())
            },

            Self::Normal(range, factor, wf) => {
                let i = if BACK {
                    range.next_back()?
                }
                else {
                    range.next()?
                };

                let x = *factor * F::from(i).unwrap();
                let y = wf.calc(x);

                Some(y)
            },
        }
    }
}

impl<W, F, const SYMM: bool> Iterator for IterImpl<W, F, SYMM>
where
    W: Window<F>,
    F: Float,
{
    type Item = F;

    fn next(&mut self) -> Option<Self::Item> {
        self.advance::<NO_BACK>()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::ZeroOne(it) => it.size_hint(),
            Self::Normal(range, ..) => range.size_hint(),
        }
    }
}

impl<W, F, const SYMM: bool> ExactSizeIterator for IterImpl<W, F, SYMM>
where
    W: Window<F>,
    F: Float,
{
    fn len(&self) -> usize {
        match self {
            Self::ZeroOne(it) => it.len(),
            Self::Normal(range, ..) => range.len(),
        }
    }
}

impl<W, F, const SYMM: bool> DoubleEndedIterator for IterImpl<W, F, SYMM>
where
    W: Window<F>,
    F: Float,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.advance::<DO_BACK>()
    }
}

/// An [`Iterator`] that yields the values of a window (via a [`Window`])
/// for a given number of points, evenly spaced to span the interval [0.0, 1.0].
pub struct Iter<W, F>(IterImpl<W, F, DO_SYMM>)
where
    W: Window<F>,
    F: Float,
;

impl<W, F> Iterator for Iter<W, F>
where
    W: Window<F>,
    F: Float,
{
    type Item = F;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<W, F> ExactSizeIterator for Iter<W, F>
where
    W: Window<F>,
    F: Float,
{
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<W, F> DoubleEndedIterator for Iter<W, F>
where
    W: Window<F>,
    F: Float,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

/// An [`Iterator`] that yields the first `N` values of an [`Iter`] with
/// `N + 1` points.
///
/// This produces a periodic, asymmetric version of the window, used in cases
/// when the window needs to be repeated.
pub struct IterPeriodic<W, F>(IterImpl<W, F, NO_SYMM>)
where
    W: Window<F>,
    F: Float,
;

impl<W, F> Iterator for IterPeriodic<W, F>
where
    W: Window<F>,
    F: Float,
{
    type Item = F;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<W, F> ExactSizeIterator for IterPeriodic<W, F>
where
    W: Window<F>,
    F: Float,
{
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<W, F> DoubleEndedIterator for IterPeriodic<W, F>
where
    W: Window<F>,
    F: Float,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}
