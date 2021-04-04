use core::f64::consts::PI;

use num_traits::Float;

use crate::{Duplex, Frame, Sample, Signal};
use crate::buffer::{Buffer, Fixed};
use crate::interpolate::Interpolator;

trait SincOp {
    fn sinc(self) -> Self;
}

impl<X> SincOp for X
where
    X: Float,
{
    #[inline]
    fn sinc(self) -> Self {
        if self.is_zero() {
            Self::one()
        } else {
            self.sin() / self
        }
    }
}

/// An [`Interpolator`] that uses sinc interpolation on a window of [`Frame`]s.
///
/// One of the better sample rate converters, although it uses significantly
/// more computation.
///
/// ```
/// use sampara::interpolate::{Sinc, Interpolator};
///
/// fn main() {
///     let sinc = Sinc::new([
///         [10, 15, 20, 25],
///         [20, 25, 30, 35],
///         [30, 35, 40, 45],
///         [40, 45, 50, 55],
///     ]);
///     assert_eq!(sinc.interpolate(0.00), [10, 15, 20, 25]);
///     assert_eq!(sinc.interpolate(0.25), [12, 17, 23, 28]);
///     assert_eq!(sinc.interpolate(0.50), [15, 21, 26, 32]);
///     assert_eq!(sinc.interpolate(0.75), [19, 24, 29, 35]);
/// }
/// ```
pub struct Sinc<F, B, const N: usize>
where
    B: Buffer<Item = F>,
    F: Frame<N>,
{
    buffer: Fixed<B>,
    idx: usize,
}

impl<F, B, const N: usize> Sinc<F, B, N>
where
    B: Buffer<Item = F>,
    F: Frame<N>,
{
    /// Creates a new [`Sinc`] interpolator with a given working [`Buffer`].
    ///
    /// The given [`Buffer`] should have a length that is double the desired
    /// sinc interpolation "depth".
    ///
    /// The initial contents of the [`Buffer`] will be used as padding for the
    /// interpolated signal.
    ///
    /// Panics if the length of the given [`Buffer`] is not a multiple of 2.
    pub fn new(buffer: B) -> Self {
        // TODO: Is this needed?
        assert!(buffer.as_ref().len() % 2 == 0);

        Self {
            buffer: Fixed::from(buffer),
            idx: 0,
        }
    }

    #[inline]
    fn depth(&self) -> usize {
        self.buffer.capacity() / 2
    }
}

impl<F, B, const N: usize> Interpolator<N> for Sinc<F, B, N>
where
    B: Buffer<Item = F>,
    F: Frame<N>,
    F::Sample: Duplex<f64>,
{
    type Frame = F;

    fn interpolate(&self, x: f64) -> Self::Frame {
        let phil = x;
        let phir = 1.0 - x;
        let nl = self.idx;
        let nr = self.idx + 1;
        let depth = self.depth();

        let rightmost = nl + depth;
        let leftmost = nr as isize - depth as isize;
        let max_depth = if rightmost >= self.buffer.capacity() {
            self.buffer.capacity() - depth
        } else if leftmost < 0 {
            (depth as isize + leftmost) as usize
        } else {
            depth
        };

        #[inline(always)]
        fn factor(phi: f64, n: usize, depth: usize) -> f64 {
            let a = PI * (phi + n as f64);
            let first = a.sinc();
            let second = 0.5 + 0.5 * (a / depth as f64).cos();

            first * second
        }

        let mut ret: F = Frame::EQUILIBRIUM;
        for n in 0..max_depth {
            let factor_l = factor(phil, n, depth);
            let factor_r = factor(phir, n, depth);

            ret.zip_transform(self.buffer[nl - n], |vs, r_lag| {
                let add = (factor_l * r_lag.into_sample::<f64>())
                    .into_sample::<F::Sample>()
                    .into_signed_sample();

                Sample::add_amp(vs, add)
            });

            ret.zip_transform(self.buffer[nr + n], |vs, r_lag| {
                let add = (factor_r * r_lag.into_sample::<f64>())
                    .into_sample::<F::Sample>()
                    .into_signed_sample();

                Sample::add_amp(vs, add)
            });
        }

        ret
    }

    fn advance(&mut self, next_frame: Self::Frame) {
        let _prev_frame = self.buffer.push(next_frame);
        if self.idx < self.depth() {
            self.idx += 1;
        }
    }

    fn initialize<S>(&mut self, signal: &mut S) -> Option<()>
    where
        S: Signal<N, Frame = F>
    {
        for b in self.buffer.iter_mut() {
            *b = signal.next()?;
        }

        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sinc_f32() {
        let inputs_expected = [
            (0.0f32, 1.0f32),
            (1.0, 0.84147096),
            (2.0, 0.45464870),
            (3.0, 0.04704),
            (-1.0, 0.84147096),
            (-2.0, 0.45464870),
            (-3.0, 0.04704),
        ];

        for (input, expected) in inputs_expected.iter() {
            assert_eq!(input.sinc(), *expected);
        }
    }

    #[test]
    fn sinc_f64() {
        let inputs_expected = [
            (0.0f64, 1.0f64),
            (1.0, 0.8414709848078965),
            (2.0, 0.45464871341284085),
            (3.0, 0.0470400026866224),
            (-1.0, 0.8414709848078965),
            (-2.0, 0.45464871341284085),
            (-3.0, 0.0470400026866224),
        ];

        for (input, expected) in inputs_expected.iter() {
            assert_eq!(input.sinc(), *expected);
        }
    }
}
