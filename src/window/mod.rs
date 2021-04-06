use num_traits::Float;

use crate::buffer::Buffer;

pub trait WindowFunc<F: Float> {
    /// Given a value in the interval [-1.0, 1.0], returns the value of the
    /// window function at that point.
    fn calc(x: F) -> F;

    // fn fill_buffer<B>(buffer: &mut B)
    // where
    //     B: Buffer<Item = F>,
    // {
    //     todo!();
    // }

    fn iter(len: usize) -> Iter<Self, F>
    where
        Self: Sized,
    {
        Iter {
            i: 0,
            len,
            _marker: Default::default(),
        }
    }
}

pub struct Rectangle;

impl<F: Float> WindowFunc<F> for Rectangle {
    fn calc(_x: F) -> F {
        F::one()
    }
}

pub struct Triangle;

impl<F: Float> WindowFunc<F> for Triangle {
    fn calc(x: F) -> F {
        F::one() - x.abs()
    }
}

/// An [`Iterator`] that yields the values of a window (via a [`WindowFunc`])
/// for a given number of points, evenly spaced to exactly span the interval
/// [-1.0, 1.0].
///
/// Iterating over 0 points yields no values. Iterating over 1 point yields 0.0
/// once, regardless of the chosen [`WindowFunc`].
///
/// ```
/// use sampara::window::{WindowFunc, Triangle, Iter};
///
/// fn main() {
///     // An odd number of points produces a value at `x = 0.0` exactly.
///     let mut iter = Triangle::iter(5);
///
///     assert_eq!(iter.next(), Some(0.0f64));
///     assert_eq!(iter.next(), Some(0.5));
///     assert_eq!(iter.next(), Some(1.0)); // x = 0.0
///     assert_eq!(iter.next(), Some(0.5));
///     assert_eq!(iter.next(), Some(0.0));
///     assert_eq!(iter.next(), None);
///
///     // An even number of points misses the `x = 0.0` point slightly.
///     let mut iter = Triangle::iter(4);
///
///     assert_eq!(iter.next(), Some(0.0f64));
///     assert_eq!(iter.next(), Some(0.6666666666666666));
///     assert_eq!(iter.next(), Some(0.6666666666666667));
///     assert_eq!(iter.next(), Some(0.0));
///     assert_eq!(iter.next(), None);
///
///     // Two points produce values at `x = -1.0` and `x = 1.0`.
///     let mut iter = Triangle::iter(2);
///
///     assert_eq!(iter.next(), Some(0.0f64));
///     assert_eq!(iter.next(), Some(0.0));
///     assert_eq!(iter.next(), None);
///
///     // One point always yields a value of `0.0`.
///     let mut iter = Triangle::iter(1);
///
///     assert_eq!(iter.next(), Some(0.0f64));
///     assert_eq!(iter.next(), None);
///
///     // Zero points is an empty iterator.
///     let mut iter: Iter<_, f64> = Triangle::iter(0);
///
///     assert_eq!(iter.next(), None);
/// }
/// ```
pub struct Iter<W, F>
where
    W: WindowFunc<F>,
    F: Float,
{
    i: usize,
    len: usize,
    _marker: std::marker::PhantomData<(W, F)>,
}

impl<W, F> Iterator for Iter<W, F>
where
    W: WindowFunc<F>,
    F: Float,
{
    type Item = F;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.len {
            let y = match self.len {
                0 => unreachable!(),

                // TODO: Should this be zero or one?
                1 => F::zero(),

                n => {
                    let f = F::from(2).unwrap() / F::from(n - 1).unwrap();

                    let x = f * F::from(self.i).unwrap() - F::one();

                    W::calc(x)
                },
            };

            self.i += 1;
            Some(y)
        }
        else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // This should never underflow.
        let r = self.len - self.i;
        (r, Some(r))
    }
}

impl<W, F> ExactSizeIterator for Iter<W, F>
where
    W: WindowFunc<F>,
    F: Float,
{}
