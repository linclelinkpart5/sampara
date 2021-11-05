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
        let a0 = F::from(25.0 / 46.0).unwrap();
        let a1 = F::one() - a0;

        a0 + a1 * (F::TAU() * x).cos()
    }
}

/// Represents a Bartlett-Hann window.
pub struct BartlettHann;

impl<F: FloatSample> Window<F> for BartlettHann {
    fn calc(&self, x: F) -> F {
        let c0 = F::from(0.62).unwrap();
        let c1 = F::from(0.48).unwrap();
        let c2 = F::from(0.38).unwrap();
        let half = F::from(0.5).unwrap();

        c0 - c1 * (x - half).abs() + c2 * (F::TAU() * (x - half)).cos()
    }
}

/// Represents a Bohman window.
pub struct Bohman;

impl<F: FloatSample> Window<F> for Bohman {
    fn calc(&self, x: F) -> F {
        let sx = ((x + x) - F::one()).abs();

        (F::one() - sx) * (F::PI() * sx).cos() + F::FRAC_1_PI() * (F::PI() * sx).sin()
    }
}

/// Represents a Blackman window.
pub struct Blackman;

impl<F: FloatSample> Window<F> for Blackman {
    fn calc(&self, x: F) -> F {
        const A: f64 = 0.16;

        let a0 = F::from(0.5 * (1.0 - A)).unwrap();
        let a1 = F::from(0.5).unwrap();
        let a2 = F::from(0.5 * A).unwrap();

        let c1 = (F::TAU() * x).cos();
        let c2 = ((F::TAU() + F::TAU()) * x).cos();

        a0 - a1 * c1 + a2 * c2
    }
}

/// Represents an "exact" Blackman window.
pub struct BlackmanExact;

impl<F: FloatSample> Window<F> for BlackmanExact {
    fn calc(&self, x: F) -> F {
        let a0 = F::from(7938.0 / 18608.0).unwrap();
        let a1 = F::from(9240.0 / 18608.0).unwrap();
        let a2 = F::from(1430.0 / 18608.0).unwrap();

        let c1 = (F::TAU() * x).cos();
        let c2 = ((F::TAU() + F::TAU()) * x).cos();

        a0 - a1 * c1 + a2 * c2
    }
}

/// Represents a Blackman-Harris window.
pub struct BlackmanHarris;

impl<F: FloatSample> Window<F> for BlackmanHarris {
    fn calc(&self, x: F) -> F {
        let a0 = F::from(0.35875).unwrap();
        let a1 = F::from(0.48829).unwrap();
        let a2 = F::from(0.14128).unwrap();
        let a3 = F::from(0.01168).unwrap();

        let c1 = (F::TAU() * x).cos();
        let c2 = ((F::TAU() + F::TAU()) * x).cos();
        let c3 = ((F::TAU() + F::TAU() + F::TAU()) * x).cos();

        a0 - a1 * c1 + a2 * c2 - a3 * c3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::f64::consts::{FRAC_1_PI, PI, TAU};

    use proptest::prelude::*;

    fn arb_delta() -> impl Strategy<Value = f64> {
        (1u64..)
            .prop_flat_map(|len| (Just(len), 0..=len))
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
        fn prop_bartlett_hann(x in arb_delta()) {
            let wf = BartlettHann;

            const C0: f64 = 0.62;
            const C1: f64 = 0.48;
            const C2: f64 = 0.38;

            let expected = C0 - C1 * (x - 0.5).abs() + C2 * (TAU * (x - 0.5)).cos();
            let produced = wf.calc(x);

            assert_eq!(expected, produced);
        }

        #[test]
        fn prop_bohman(x in arb_delta()) {
            let wf = Bohman;

            let n = (2.0 * x - 1.0).abs();

            let expected = (1.0 - n) * (PI * n).cos() + FRAC_1_PI * (PI * n).sin();
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

        #[test]
        fn prop_blackman_harris(x in arb_delta()) {
            let wf = BlackmanHarris;

            const A0: f64 = 0.35875;
            const A1: f64 = 0.48829;
            const A2: f64 = 0.14128;
            const A3: f64 = 0.01168;

            let c1 = (TAU * x).cos();
            let c2 = (2.0 * TAU * x).cos();
            let c3 = (3.0 * TAU * x).cos();

            let expected = A0 - A1 * c1 + A2 * c2 - A3 * c3;
            let produced = wf.calc(x);

            assert_eq!(expected, produced);
        }
    }
}
