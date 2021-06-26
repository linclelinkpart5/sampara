// All of these are derived from https://en.wikipedia.org/wiki/Window_function

use crate::sample::FloatSample;
use crate::window::Window;

/// Represents a rectangular (aka boxcar) window.
pub struct Rectangle;

impl<F: FloatSample> Window<F> for Rectangle {
    fn calc(&self, _x: F) -> F {
        F::one()
    }
}

/// Represents a triangular window.
pub struct Triangle;

impl<F: FloatSample> Window<F> for Triangle {
    fn calc(&self, x: F) -> F {
        F::one() - ((x + x) - F::one()).abs()
    }
}

/// Represents a cosine window.
pub struct Cosine;

impl<F: FloatSample> Window<F> for Cosine {
    fn calc(&self, x: F) -> F {
        (F::PI() * x).sin()
    }
}

/// Represents a Welch window.
pub struct Welch;

impl<F: FloatSample> Window<F> for Welch {
    fn calc(&self, x: F) -> F {
        let i = F::from(2.0).unwrap() * x - F::one();
        F::one() - (i * i)
    }
}

/// Represents a Hann window.
pub struct Hann;

impl<F: FloatSample> Window<F> for Hann {
    fn calc(&self, x: F) -> F {
        (F::one() - (F::TAU() * x).cos()) * F::from(0.5).unwrap()
    }
}

/// Represents a Hamming window.
pub struct Hamming;

impl<F: FloatSample> Window<F> for Hamming {
    fn calc(&self, x: F) -> F {
        const A0: f64 = 25.0 / 46.0;
        const A1: f64 = 1.0 - A0;

        let a0 = F::from(A0).unwrap();
        let a1 = F::from(A1).unwrap();

        a0 + a1 * (F::TAU() * x).cos()
    }
}

/// Represents a Blackman window.
pub struct Blackman;

impl<F: FloatSample> Window<F> for Blackman {
    fn calc(&self, x: F) -> F {
        const A: f64 = 0.16;
        const A0: f64 = 0.5 * (1.0 - A);
        const A1: f64 = 0.5;
        const A2: f64 = 0.5 * A;

        let a0 = F::from(A0).unwrap();
        let a1 = F::from(A1).unwrap();
        let a2 = F::from(A2).unwrap();

        let c1 = (F::TAU() * x).cos();
        let c2 = ((F::TAU() + F::TAU()) * x).cos();

        a0 - a1 * c1 + a2 * c2
    }
}

/// Represents an "exact" Blackman window.
pub struct BlackmanExact;

impl<F: FloatSample> Window<F> for BlackmanExact {
    fn calc(&self, x: F) -> F {
        const A0: f64 = 7938.0 / 18608.0;
        const A1: f64 = 9240.0 / 18608.0;
        const A2: f64 = 1430.0 / 18608.0;

        let a0 = F::from(A0).unwrap();
        let a1 = F::from(A1).unwrap();
        let a2 = F::from(A2).unwrap();

        let c1 = (F::TAU() * x).cos();
        let c2 = ((F::TAU() + F::TAU()) * x).cos();

        a0 - a1 * c1 + a2 * c2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::f64::consts::{PI, TAU};

    use proptest::prelude::*;

    fn arb_delta() -> impl Strategy<Value = f64> {
        (1u64..)
        .prop_flat_map(|len| {
            (Just(len), 0..=len)
        })
        .prop_map(|(len, idx)| idx as f64 / len as f64)
    }

    proptest! {
        #[test]
        fn prop_rectangle(x in arb_delta()) {
            let wf = Rectangle;

            let expected = 1.0;
            let produced = wf.calc(x);

            assert_eq!(expected, produced);
        }

        #[test]
        fn prop_triangle(x in arb_delta()) {
            let wf = Triangle;

            let expected = 1.0 - (2.0 * x - 1.0).abs();
            let produced = wf.calc(x);

            assert_eq!(expected, produced);
        }

        #[test]
        fn prop_cosine(x in arb_delta()) {
            let wf = Cosine;

            let expected = (PI * x).sin();
            let produced = wf.calc(x);

            assert_eq!(expected, produced);
        }

        #[test]
        fn prop_welch(x in arb_delta()) {
            let wf = Welch;

            let i = 2.0 * x - 1.0;
            let expected = 1.0 - (i * i);
            let produced = wf.calc(x);

            assert_eq!(expected, produced);
        }

        #[test]
        fn prop_hann(x in arb_delta()) {
            let wf = Hann;

            let expected = (1.0 - (TAU * x).cos()) * 0.5;
            let produced = wf.calc(x);

            assert_eq!(expected, produced);
        }

        #[test]
        fn prop_hamming(x in arb_delta()) {
            let wf = Hamming;

            const A0: f64 = 25.0 / 46.0;
            const A1: f64 = 1.0 - A0;

            let expected = A0 + A1 * (TAU * x).cos();
            let produced = wf.calc(x);

            assert_eq!(expected, produced);
        }

        #[test]
        fn prop_blackman(x in arb_delta()) {
            let wf = Blackman;

            const A: f64 = 0.16;
            const A0: f64 = 0.5 * (1.0 - A);
            const A1: f64 = 0.5;
            const A2: f64 = 0.5 * A;

            let c1 = (TAU * x).cos();
            let c2 = (2.0 * TAU * x).cos();

            let expected = A0 - A1 * c1 + A2 * c2;
            let produced = wf.calc(x);

            assert_eq!(expected, produced);
        }

        #[test]
        fn prop_blackman_exact(x in arb_delta()) {
            let wf = BlackmanExact;

            const A0: f64 = 7938.0 / 18608.0;
            const A1: f64 = 9240.0 / 18608.0;
            const A2: f64 = 1430.0 / 18608.0;

            let c1 = (TAU * x).cos();
            let c2 = (2.0 * TAU * x).cos();

            let expected = A0 - A1 * c1 + A2 * c2;
            let produced = wf.calc(x);

            assert_eq!(expected, produced);
        }
    }
}
