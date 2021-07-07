use num_traits::{Float, NumCast};

use crate::{Sample, Frame};
use crate::sample::FloatSample;

const DO_SQRT: bool = true;
const NO_SQRT: bool = false;
const DO_POW2: bool = true;
const NO_POW2: bool = false;

struct SummageInner<F, const N: usize, const SQRT: bool, const POW2: bool>
where
    F: Frame<N>,
    F::Sample: FloatSample,
{
    avg: F,
    count: u64,
}

impl<F, const N: usize, const SQRT: bool, const POW2: bool> SummageInner<F, N, SQRT, POW2>
where
    F: Frame<N>,
    F::Sample: FloatSample,
{
    #[inline]
    fn __advance(&mut self, input: F) {
        let mut input = input;

        if POW2 {
            // Calculate the square of the new frame and push onto the buffer.
            input.transform(|x| x * x);
        }

        self.count += 1;
        let c = <F::Sample as NumCast>::from(self.count).unwrap();
        self.avg.zip_transform(input, |a, x| {
            let mut new_a = a + (x - a) / c;
            if SQRT {
                // In case of floating point rounding errors, floor at equilibrium.
                new_a = new_a.max(Sample::EQUILIBRIUM);
            }
            new_a
        });
    }

    #[inline]
    fn __current(&self) -> F {
        if SQRT {
            self.avg.apply(Float::sqrt)
        }
        else {
            self.avg
        }
    }

    #[inline]
    fn __process(&mut self, input: F) -> F {
        self.__advance(input);
        self.__current()
    }
}

struct MinMaxInner<F, const N: usize, const MAX: bool>
where
    F: Frame<N>,
{
    frontier: F,
}
