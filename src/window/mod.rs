pub mod types;

pub use types::*;

use std::ops::Range;
use std::option::IntoIter as OptionIntoIter;

use num_traits::Float;

use crate::buffer::Buffer;

enum End {
    Front,
    Back,
}

pub trait Window<F: Float> {
    /// Given a value in the interval [0.0, 1.0], returns the value of the
    /// window function at that point.
    fn calc(&self, x: F) -> F;

    /// Returns an iterator that yields the values of a window of length `N`,
    /// evenly spaced to span the input interval [0.0, 1.0].
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

    /// Fills a buffer of length `N` with the values of a window of length `N`.
    ///
    /// ```
    /// use sampara::window::Window;
    /// use sampara::window::types::Triangle;
    ///
    /// fn main() {
    ///     let mut buffer = [-1.0f64; 4];
    ///     Window::fill_buffer(Triangle, &mut buffer);
    ///     assert_eq!(buffer, [0.0, 0.6666666666666666, 0.6666666666666667, 0.0]);
    ///
    ///     let mut buffer = [-1.0f64; 1];
    ///     Window::fill_buffer(Triangle, &mut buffer);
    ///     assert_eq!(buffer, [1.0]);
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
                let factor = F::from(n - 1).unwrap().recip();
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

                opt.map(|_| F::one())
            },

            Self::Normal(range, factor, wf) => {
                let i = match end {
                    End::Front => range.next(),
                    End::Back => range.next_back(),
                }?;

                let x = *factor * F::from(i).unwrap();
                let y = wf.calc(x);

                Some(y)
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
/// for a given number of points, evenly spaced to span the interval [0.0, 1.0].
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
