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
}
