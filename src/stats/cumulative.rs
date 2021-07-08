use num_traits::{Float, NumCast};

use crate::{Sample, Frame};
use crate::sample::FloatSample;

const DO_SQRT: bool = true;
const NO_SQRT: bool = false;
const DO_POW2: bool = true;
const NO_POW2: bool = false;

const DO_MAX: bool = true;
const DO_MIN: bool = false;

struct SummageInner<F, const N: usize, const SQRT: bool, const POW2: bool>
where
    F: Frame<N>,
    F::Sample: FloatSample,
{
    avg: F,
    count: u64,
}

impl<F, const N: usize, const SQRT: bool, const POW2: bool> Default for SummageInner<F, N, SQRT, POW2>
where
    F: Frame<N>,
    F::Sample: FloatSample,
{
    fn default() -> Self {
        Self {
            avg: Frame::EQUILIBRIUM,
            count: 0,
        }
    }
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

type RmsInner<F, const N: usize> = SummageInner<F, N, DO_SQRT, DO_POW2>;
type MsInner<F, const N: usize> = SummageInner<F, N, NO_SQRT, DO_POW2>;
type MeanInner<F, const N: usize> = SummageInner<F, N, NO_SQRT, NO_POW2>;

struct ExtremaInner<F, const N: usize, const MAX: bool>
where
    F: Frame<N>,
{
    // NOTE: No need to wrap this in an `Option`, since this is an internal
    //       class. We can just make sure to never return this until at least
    //       one frame has been processed.
    extrema: F,
    is_empty: bool,
}

impl<F, const N: usize, const MAX: bool> Default for ExtremaInner<F, N, MAX>
where
    F: Frame<N>,
{
    fn default() -> Self {
        Self {
            extrema: Frame::EQUILIBRIUM,
            is_empty: false,
        }
    }
}

impl<F, const N: usize, const MAX: bool> ExtremaInner<F, N, MAX>
where
    F: Frame<N>,
{
    #[inline]
    fn __advance(&mut self, input: F) {
        if self.is_empty {
            self.extrema = input;
            self.is_empty = false;
        }
        else {
            self.extrema.zip_transform(input, |e, x| {
                if crate::stats::surpasses::<_, MAX>(&x, &e) {
                    x
                }
                else {
                    e
                }
            });
        }
    }

    #[inline]
    fn __current(&self) -> F {
        self.extrema
    }

    #[inline]
    fn __process(&mut self, input: F) -> F {
        self.__advance(input);
        self.__current()
    }
}

type MinInner<F, const N: usize> = ExtremaInner<F, N, DO_MIN>;
type MaxInner<F, const N: usize> = ExtremaInner<F, N, DO_MAX>;

#[cfg(test)]
mod tests {
    use super::*;

    use proptest::prelude::*;
    use approx::assert_relative_eq;

    const N: usize = 16;

    fn arb_frame() -> impl Strategy<Value = [f32; N]> {
        prop::array::uniform16(-10000.0f32..=10000.0)
        // prop::array::uniform16(any::<f32>())
    }

    fn arb_input_feed() -> impl Strategy<Value = Vec<[f32; N]>> {
        prop::collection::vec(arb_frame(), 1..=32)
    }

    proptest! {
        #[test]
        fn prop_rms_inner(in_feed in arb_input_feed()) {
            let mut inner = RmsInner::<[f32; N], N>::default();

            // NOTE: Older less-numerically stable version for comparison.
            // let expected: [f32; N] = {
            //     let mut exp: [f32; N] = Frame::EQUILIBRIUM;

            //     for frame in in_feed.iter().copied() {
            //         exp.zip_transform(frame, |e, x| e + x * x);
            //     }

            //     let len_f = in_feed.len() as f32;

            //     exp.apply(|x| Float::sqrt(x / len_f))
            // };

            let expected: [f32; N] = {
                let mut exp: [f32; N] = Frame::EQUILIBRIUM;

                for (frame, count) in in_feed.iter().copied().zip(1..) {
                    exp.zip_transform(frame, |e, x| {
                        let x = x * x;
                        e + (x - e) / count as f32
                    });
                }

                exp.apply(Float::sqrt)
            };

            for frame in in_feed {
                inner.__advance(frame);
            }

            let produced = inner.__current();

            assert_relative_eq!(produced.as_slice(), expected.as_slice());
        }
    }
}
