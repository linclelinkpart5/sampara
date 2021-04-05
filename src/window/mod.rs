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
