use gcd::Gcd;
use thiserror::Error;

use crate::sample::FloatSample;

/// A bounded floating point value that cycles in the interval [0.0, 1.0).
pub trait Phase {
    type Step: FloatSample;

    /// Advances the phase to the next value, while also keeping track of how
    /// many times the phase was wrapped back to 0.0 due to it being >= 1.0.
    /// Returns the number of wraps that were performed.
    fn advance_count(&mut self) -> u32;

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

    fn advance_count(&mut self) -> u32 {
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

enum Maxed {
    Num,
    Den,
}

/// Helper method to co-reduce two "add" and "rem" factors.
fn simplify(to_add: u32, to_rem: u32) -> (u32, u32) {
    let (maxed, normal) = {
        // If the factors are equal, reduce to no-op.
        // NOTE: This also handles the case of both factors equalling `MAX`.
        if to_add == to_rem {
            return (0, 0);
        }
        // Check if the add factor is `MAX`.
        else if to_add == u32::MAX {
            (Maxed::Num, to_rem + 1)
        }
        // Check if the rem factor is `MAX`.
        else if to_rem == u32::MAX {
            (Maxed::Den, to_add + 1)
        }
        // Simple case, convert the factors to *-ators by adding 1, simplify by
        // using the GCD, and convert back to factors by subtracting 1.
        else {
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
    // `MAX + 1`. We assume that this value is a perfect power of 2, meaning it
    // is only divisible by smaller powers of 2. Thus, find the largest power
    // of 2 that divides the non-overflowed *-ator, which will be the GCD for
    // this simplification.
    debug_assert!(normal > 0);
    let div_pow_2 = normal.trailing_zeros();

    if div_pow_2 == 0 {
        // There is no way to simplify, so this is in lowest terms already.
        return (to_add, to_rem);
    }

    // Use the GCD and the fact that it is a power of 2 to simplify the *-ators.
    let shl_n = u32::BITS - div_pow_2;
    let simp_overflow = 1u32 << shl_n;
    let simp_normal = normal >> div_pow_2;

    debug_assert!(simp_normal > 0);
    debug_assert!(simp_overflow > 0);

    match maxed {
        Maxed::Num => (simp_overflow - 1, simp_normal - 1),
        Maxed::Den => (simp_normal - 1, simp_overflow - 1),
    }
}

pub struct Rational<X: FloatSample> {
    // NOTE: If there existed a `u33` type, that could be used instead.
    i: u64,
    max_value: u32,
    skip_extra: u32,
    _marker: std::marker::PhantomData<X>,
}

impl<X: FloatSample> Rational<X> {
    pub fn new(to_add: u32, to_rem: u32) -> Self {
        let (to_add, to_rem) = simplify(to_add, to_rem);

        Self {
            i: 0,
            max_value: to_add,
            skip_extra: to_rem,
            _marker: Default::default(),
        }
    }
}

impl<X: FloatSample> Phase for Rational<X> {
    type Step = X;

    fn advance_count(&mut self) -> u32 {
        debug_assert!(self.i <= self.max_value as u64);

        let adv_i = self.i + self.skip_extra as u64 + 1;
        let div = self.max_value as u64 + 1;

        self.i = adv_i % div;
        let num_loops = adv_i / div;

        assert!(num_loops <= u32::MAX as u64);

        num_loops as u32
    }

    fn current(&self) -> Self::Step {
        debug_assert!(self.i <= self.max_value as u64);

        if self.i == 0 {
            X::zero()
        } else {
            X::from(self.i).unwrap() / X::from(self.max_value as u64 + 1).unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use proptest::prelude::*;

    const MAX_DELTA: f32 = 16.0;
    const NUM_STEPS: u32 = 1000;

    proptest! {
        #[test]
        fn simplify_is_symmetric(to_add in any::<u32>(), to_rem in any::<u32>()) {
            let produced = {
                let (a, b) = simplify(to_rem, to_add);
                (b, a)
            };
            let expected = simplify(to_add, to_rem);

            assert_eq!(produced, expected);
        }

        #[test]
        fn simplify_simplifies(to_add in any::<u32>(), to_rem in any::<u32>()) {
            let produced = {
                let (simp_to_add, simp_to_rem) = simplify(to_add, to_rem);
                (simp_to_add as u64, simp_to_rem as u64)
            };

            let (num, den) = (to_add as u64 + 1, to_rem as u64 + 1);
            let div = num.gcd(den);

            let (simp_num, simp_den) = (num / div, den / div);
            let expected = (simp_num - 1, simp_den - 1);

            assert_eq!(produced, expected);
        }

        #[test]
        fn simplify_handles_max(exp in 0..u32::BITS) {
            let max = u32::MAX;
            let min = u32::MAX >> exp;

            let factor = 2u32.pow(exp);

            let produced = simplify(max, min);
            let expected = (factor - 1, 0);
            assert_eq!(produced, expected);

            let produced = simplify(min, max);
            let expected = (0, factor - 1);
            assert_eq!(produced, expected);
        }

        #[test]
        // TODO: Replace range with `(Bound::Excluded, Bound::Included)` when
        //       available in `proptest`.
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
        fn rational_happy_path(to_add in any::<u32>(), to_rem in any::<u32>()) {
            let mut phase = Rational::<f32>::new(to_add, to_rem);

            let (simp_to_add, simp_to_rem) = simplify(to_add, to_rem);

            let mut i = 0;
            for _ in 0..NUM_STEPS {
                let adv_i = i + simp_to_rem as u64 + 1;
                let div = simp_to_add as u64 + 1;

                let next_i = adv_i % div;
                let num_loops = (adv_i / div) as u32;

                let x = i as f32 / (simp_to_add as u64 + 1) as f32;

                assert_eq!(phase.current(), x);
                assert_eq!(phase.advance_count(), num_loops);
                assert_eq!(phase.i, next_i);

                i = next_i;
            }
        }
    }
}
