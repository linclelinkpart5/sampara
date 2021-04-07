use std::ops::Range;
use std::option::IntoIter as OptionIntoIter;

use num_traits::Float;

use crate::buffer::Buffer;

enum End {
    Front,
    Back,
}

pub trait Window<F: Float> {
    /// Given a value in the interval [-1.0, 1.0], returns the value of the
    /// window function at that point.
    fn calc(&self, x: F) -> F;

    /// Returns an iterator that yields the values of a window of length `N`,
    /// evenly spaced to exactly span the input interval [-1.0, 1.0].
    ///
    /// A window of length 1 produces an iterator that yields a single 0.0.
    ///
    /// A window of length 0 produces an empty iterator.
    ///
    /// ```
    /// use sampara::window::{Window, Triangle};
    ///
    /// fn main() {
    ///     // An odd number of points produces a value at `x = 0.0` exactly.
    ///     let mut iter = Window::iter(Triangle, 5);
    ///
    ///     assert_eq!(iter.next(), Some(0.0f64));
    ///     assert_eq!(iter.next(), Some(0.5));
    ///     assert_eq!(iter.next(), Some(1.0)); // x = 0.0
    ///     assert_eq!(iter.next(), Some(0.5));
    ///     assert_eq!(iter.next(), Some(0.0));
    ///     assert_eq!(iter.next(), None);
    ///
    ///     // An even number of points misses the `x = 0.0` point slightly.
    ///     let mut iter = Window::iter(Triangle, 4);
    ///
    ///     assert_eq!(iter.next(), Some(0.0f64));
    ///     assert_eq!(iter.next(), Some(0.6666666666666666));
    ///     assert_eq!(iter.next(), Some(0.6666666666666667));
    ///     assert_eq!(iter.next(), Some(0.0));
    ///     assert_eq!(iter.next(), None);
    ///
    ///     // Two points produce values at `x = -1.0` and `x = 1.0`.
    ///     let mut iter = Window::iter(Triangle, 2);
    ///
    ///     assert_eq!(iter.next(), Some(0.0f64));
    ///     assert_eq!(iter.next(), Some(0.0));
    ///     assert_eq!(iter.next(), None);
    ///
    ///     // One point always yields a single value of `0.0`.
    ///     let mut iter = Window::iter(Triangle, 1);
    ///
    ///     assert_eq!(iter.next(), Some(0.0f64));
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

    /// Fills a buffer of length `N` with the values of a window of length `N`.
    ///
    /// ```
    /// use sampara::window::{Window, Triangle};
    ///
    /// fn main() {
    ///     let mut buffer = [-1.0f64; 5];
    ///     Window::fill_buffer(Triangle, &mut buffer);
    ///     assert_eq!(buffer, [0.0, 0.5, 1.0, 0.5, 0.0]);
    ///
    ///     let mut buffer = [-1.0f64; 4];
    ///     Window::fill_buffer(Triangle, &mut buffer);
    ///     assert_eq!(buffer, [0.0, 0.6666666666666666, 0.6666666666666667, 0.0]);
    ///
    ///     let mut buffer = [-1.0f64; 2];
    ///     Window::fill_buffer(Triangle, &mut buffer);
    ///     assert_eq!(buffer, [0.0, 0.0]);
    ///
    ///     let mut buffer = [-1.0f64; 1];
    ///     Window::fill_buffer(Triangle, &mut buffer);
    ///     assert_eq!(buffer, [0.0]);
    ///
    ///     let mut buffer = [-1.0f64; 0];
    ///     Window::fill_buffer(Triangle, &mut buffer);
    ///     assert_eq!(buffer, []);
    /// }
    /// ```
    fn fill_buffer<B>(self, buffer: &mut B)
    where
        Self: Sized,
        B: Buffer<Item = F>,
    {
        let slice = buffer.as_mut();
        let iter = self.iter(slice.len());

        for (y, b) in iter.zip(slice.iter_mut()) {
            *b = y;
        }
    }
}

pub struct Rectangle;

impl<F: Float> Window<F> for Rectangle {
    fn calc(&self, _x: F) -> F {
        F::one()
    }
}

pub struct Triangle;

impl<F: Float> Window<F> for Triangle {
    fn calc(&self, x: F) -> F {
        F::one() - x.abs()
    }
}

#[inline]
fn calc_at<W, F>(i: usize, factor: F, wf: &W) -> F
where
    W: Window<F>,
    F: Float,
{
    let x = factor * F::from(i).unwrap() - F::one();
    wf.calc(x)
}

enum IterImpl<W, F>
where
    W: Window<F>,
    F: Float,
{
    ZeroOne(OptionIntoIter<()>),
    Normal(Range<usize>, F, W),
}

impl<W, F> IterImpl<W, F>
where
    W: Window<F>,
    F: Float,
{
    fn new(len: usize, windower: W) -> Self {
        match len {
            0 => Self::ZeroOne(None.into_iter()),
            1 => Self::ZeroOne(Some(()).into_iter()),
            n => {
                let factor = F::from(2).unwrap() / F::from(n - 1).unwrap();
                Self::Normal(0..n, factor, windower)
            },
        }
    }

    #[inline]
    fn advance(&mut self, end: End) -> Option<<Self as Iterator>::Item> {
        match self {
            Self::ZeroOne(it) => {
                let opt = match end {
                    End::Front => it.next(),
                    End::Back => it.next_back(),
                };

                // TODO: Should this be zero or one?
                opt.map(|_| F::zero())
            },

            Self::Normal(range, factor, wf) => {
                let i = match end {
                    End::Front => range.next(),
                    End::Back => range.next_back(),
                }?;

                Some(calc_at(i, *factor, wf))
            },
        }
    }
}

impl<W, F> Iterator for IterImpl<W, F>
where
    W: Window<F>,
    F: Float,
{
    type Item = F;

    fn next(&mut self) -> Option<Self::Item> {
        self.advance(End::Front)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::ZeroOne(it) => it.size_hint(),
            Self::Normal(range, ..) => range.size_hint(),
        }
    }
}

impl<W, F> ExactSizeIterator for IterImpl<W, F>
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

impl<W, F> DoubleEndedIterator for IterImpl<W, F>
where
    W: Window<F>,
    F: Float,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.advance(End::Back)
    }
}

/// An [`Iterator`] that yields the values of a window (via a [`Window`])
/// for a given number of points, evenly spaced to exactly span the interval
/// [-1.0, 1.0].
///
/// Iterating over 0 points yields no values. Iterating over 1 point yields 0.0
/// once, regardless of the chosen [`Window`].
pub struct Iter<W, F>(IterImpl<W, F>)
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
