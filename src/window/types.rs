// All of these are derived from https://en.wikipedia.org/wiki/Window_function

use num_traits::{Float, FloatConst};

use crate::window::Window;

pub struct Rectangle;

impl<F: Float> Window<F> for Rectangle {
    fn calc(&self, _x: F) -> F {
        F::one()
    }
}

pub struct Triangle;

impl<F: Float> Window<F> for Triangle {
    fn calc(&self, x: F) -> F {
        F::one() - x.abs()
    }
}

pub struct Welch;

impl<F: Float> Window<F> for Welch {
    fn calc(&self, x: F) -> F {
        F::one() - x * x
    }
}

pub struct Hann;

impl<F: Float + FloatConst> Window<F> for Hann {
    fn calc(&self, x: F) -> F {
        let i = (F::PI() * x).sin();
        i * i
    }
}

pub struct Blackman;

impl<F: Float + FloatConst> Window<F> for Blackman {
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
