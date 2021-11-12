use gcd::Gcd;
use thiserror::Error;

use crate::sample::FloatSample;

/// A bounded floating point value that cycles in the interval [0.0, 1.0).
pub trait Phase {
    type Step: FloatSample;

    /// Advances the phase to the next value, while also keeping track of how
    /// many times the phase was wrapped back to 0.0 due to it being >= 1.0.
    /// Returns the number of wraps that were performed.
    fn advance_count(&mut self) -> usize;

    /// Advances the phase to the next value.
    fn advance(&mut self) {
        self.advance_count();
    }

    /// Returns the current phase value.
    fn current(&self) -> Self::Step;
}

#[derive(Debug, Error)]
pub enum FixedError {
    #[error("step must be finite")]
    NotFinite,
    #[error("step must be strictly greater than zero")]
    NotPositive,
}

/// A fixed-step phase, that increments by a constant amount each iteration.
pub struct Fixed<X: FloatSample> {
    accum: X,
    delta: X,
}

impl<X: FloatSample> Fixed<X> {
    pub fn new(delta: X) -> Self {
        Self::try_new(delta).unwrap()
    }

    pub fn try_new(delta: X) -> Result<Self, FixedError> {
        if !delta.is_finite() {
            return Err(FixedError::NotFinite);
        }

        if !(delta > X::zero()) {
            return Err(FixedError::NotPositive);
        }

        Ok(Self {
            accum: X::zero(),
            delta,
        })
    }
}

impl<X: FloatSample> Phase for Fixed<X> {
    type Step = X;

    fn advance_count(&mut self) -> usize {
        debug_assert!(self.delta > X::zero());
        debug_assert!(self.accum >= X::zero());
        debug_assert!(self.accum < X::one());

        self.accum = self.accum + self.delta;

        let mut frames_to_adv = 0;

        while self.accum >= X::one() {
            self.accum = self.accum - X::one();
            frames_to_adv += 1;
        }

        frames_to_adv
    }

    fn current(&self) -> Self::Step {
        self.accum
    }
}

#[derive(Debug, Error)]
pub enum RationalError {
    #[error("denominator must be greater than zero")]
    ZeroDenominator,
    #[error("numerator must be greater than zero")]
    ZeroNumerator,
}

fn simplify(to_add: usize, to_rem: usize) -> (usize, usize) {
    let (overflow_is_num, normal) = {
        if to_add == to_rem {
            return (0, 0);
        } else if to_add == usize::MAX {
            (true, to_rem + 1)
        } else if to_rem == usize::MAX {
            (false, to_add + 1)
        } else {
            let num = to_add + 1;
            let den = to_rem + 1;

            let div = num.gcd(den);

            let s_num = num / div;
            let s_den = den / div;

            debug_assert!(s_num > 0);
            debug_assert!(s_den > 0);

            return (s_num - 1, s_den - 1);
        }
    };

    // At this point, we would have an overflow of exactly one of the numerator
    // or the denominator. The "scalar" value of this *-ator would be equal to
    // `usize::MAX + 1`. We assume that this value is a perfect power of 2,
    // which means it is only divisible by smaller powers of 2. Thus, find
    // the largest power of 2 that divides the non-overflowed *-ator, that will
    // be the GCD for this simplification.
    let div_pow_2 = normal.trailing_zeros();

    if div_pow_2 == 0 {
        // There is no way to simplify, so this is in lowest terms already.
        return (to_add, to_rem);
    }

    // Use the GCD and the fact that it is a power of 2 to simplify the *-ators.
    let shl_n = usize::BITS - div_pow_2;
    let simp_overflow = 1usize << shl_n;
    let simp_normal = normal >> div_pow_2;

    if overflow_is_num {
        (simp_overflow - 1, simp_normal - 1)
    } else {
        (simp_normal - 1, simp_overflow - 1)
    }
}

pub struct Rational<X: FloatSample> {
    inter_pts_add: usize,
    after_pts_rem: usize,
    i: usize,
    _marker: std::marker::PhantomData<X>,
}

impl<X: FloatSample> Rational<X> {
    pub fn new(num: usize, den: usize) -> Self {
        Self::try_new(num, den).unwrap()
    }

    pub fn try_new(num: usize, den: usize) -> Result<Self, RationalError> {
        if den == 0 {
            return Err(RationalError::ZeroDenominator);
        }
        if num == 0 {
            return Err(RationalError::ZeroNumerator);
        }

        // Reduce the fraction.
        let div = num.gcd(den);

        let num = num / div;
        let den = den / div;

        // SAFETY: The simplified numerator and denominator should both be
        //         greater than zero at this point.
        debug_assert!(num > 0);
        debug_assert!(den > 0);
        let to_add = num - 1;
        let to_rem = den - 1;

        Ok(Self {
            inter_pts_add: to_add,
            after_pts_rem: to_rem,
            i: 0,
            _marker: Default::default(),
        })
    }

    pub fn add_rem(to_add: usize, to_rem: usize) -> Self {
        let (to_add, to_rem) = simplify(to_add, to_rem);

        Self {
            inter_pts_add: to_add,
            after_pts_rem: to_rem,
            i: 0,
            _marker: Default::default(),
        }
    }
}

impl<X: FloatSample> Phase for Rational<X> {
    type Step = X;

    fn advance_count(&mut self) -> usize {
        debug_assert!(self.i <= self.inter_pts_add);

        let mut frames_to_adv = 0;

        // NOTE: This is an inclusive end bound, so this runs (N+1) times!
        for _ in 0..=self.after_pts_rem {
            if self.i >= self.inter_pts_add {
                self.i = 0;
                frames_to_adv += 1;
            } else {
                self.i += 1;
            }
        }

        frames_to_adv
    }

    fn current(&self) -> Self::Step {
        if self.i == 0 {
            X::zero()
        } else {
            X::from(self.i).unwrap() / (X::one() + X::from(self.inter_pts_add).unwrap())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use proptest::prelude::*;

    const MAX_DELTA: f32 = 16.0;
    const MAX_TO_ADD: usize = 16;
    const MAX_TO_REM: usize = MAX_TO_ADD;
    const NUM_STEPS: usize = 1000;

    proptest! {
        #[test]
        fn simplify_is_symmetric(to_add in any::<usize>(), to_rem in any::<usize>()) {
            let produced = {
                let (a, b) = simplify(to_rem, to_add);
                (b, a)
            };
            let expected = simplify(to_add, to_rem);

            assert_eq!(produced, expected);
        }

        #[test]
        fn simplify_simplifies(to_add in any::<usize>(), to_rem in any::<usize>()) {
            let produced = {
                let (simp_to_add, simp_to_rem) = simplify(to_add, to_rem);
                (simp_to_add as u128, simp_to_rem as u128)
            };

            let (num, den) = (to_add as u128 + 1, to_rem as u128 + 1);
            let div = num.gcd(den);

            let (simp_num, simp_den) = (num / div, den / div);
            let expected = (simp_num - 1, simp_den - 1);

            assert_eq!(produced, expected);
        }

        #[test]
        fn simplify_handles_max(exp in 0..usize::BITS) {
            let max = usize::MAX;
            let min = usize::MAX >> exp;

            let factor = 2usize.pow(exp);

            let produced = simplify(max, min);
            let expected = (factor - 1, 0);
            assert_eq!(produced, expected);

            let produced = simplify(min, max);
            let expected = (0, factor - 1);
            assert_eq!(produced, expected);
        }

        #[test]
        fn fixed_happy_path(inv_delta in 0.0..MAX_DELTA) {
            let delta = MAX_DELTA - inv_delta;
            let mut accum = 0.0;

            let mut phase = Fixed::new(delta);

            for _ in 0..NUM_STEPS {
                let x = accum;

                let mut adv = 0;
                accum += delta;
                while accum >= 1.0 {
                    accum -= 1.0;
                    adv += 1;
                }

                assert_eq!(phase.current(), x);
                assert_eq!(phase.advance_count(), adv);
            }
        }

        #[test]
        fn rational_happy_path(to_add in 0usize..=MAX_TO_ADD, to_rem in 0usize..=MAX_TO_REM) {
            let mut phase = Rational::<f32>::add_rem(to_add, to_rem);

            for t in (0..).into_iter().step_by(to_rem + 1).take(NUM_STEPS) {
                let i = t % (to_add + 1);

                let x = i as f32 / (to_add + 1) as f32;

                let adv = (i + to_rem + 1) / (to_add + 1);

                assert_eq!(phase.current(), x);
                assert_eq!(phase.advance_count(), adv);
            }
        }

        #[test]
        fn rational_handles_max_add(to_rem in 0usize..=MAX_TO_REM) {
            let mut phase = Rational::<f32>::add_rem(usize::MAX, to_rem);

            assert!(NUM_STEPS < usize::MAX);

            for i in (0..).into_iter().step_by(to_rem + 1).take(NUM_STEPS) {
                let x = i as f32 / (usize::MAX as f32 + 1.0);

                assert_eq!(phase.current(), x);
                assert_eq!(phase.advance_count(), 0);
            }
        }
    }
}
