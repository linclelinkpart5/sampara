// All of these are derived from https://en.wikipedia.org/wiki/Window_function

use num_traits::{Float, FloatConst};

use crate::window::Window;

/// Represents a rectangular (aka boxcar) window.
///
/// ```
/// use sampara::window::Window;
/// use sampara::window::types::Rectangle;
///
/// fn main() {
///     let mut buffer = [-1.0; 16];
///     Window::fill(Rectangle, &mut buffer);
///     assert_eq!(buffer, [1.0; 16]);
/// }
/// ```
pub struct Rectangle;

impl<F: Float> Window<F> for Rectangle {
    fn calc(&self, _x: F) -> F {
        F::one()
    }
}

/// Represents a triangular window.
///
/// ```
/// use sampara::window::Window;
/// use sampara::window::types::Triangle;
///
/// fn main() {
///     let mut buffer = [-1.0; 16];
///     Window::fill(Triangle, &mut buffer);
///     assert_eq!(buffer, [
///         0.0000000000000000,
///         0.1333333333333333,
///         0.2666666666666666,
///         0.4000000000000000,
///         0.5333333333333333,
///         0.6666666666666666,
///         0.8000000000000000,
///         0.9333333333333333,
///         0.9333333333333333,
///         0.8000000000000000,
///         0.6666666666666667,
///         0.5333333333333334,
///         0.3999999999999999,
///         0.2666666666666666,
///         0.1333333333333333,
///         0.0000000000000000,
///     ]);
/// }
/// ```
pub struct Triangle;

impl<F: Float> Window<F> for Triangle {
    fn calc(&self, x: F) -> F {
        F::one() - ((x + x) - F::one()).abs()
    }
}

/// Represents a Welch window.
///
/// ```
/// use sampara::window::Window;
/// use sampara::window::types::Welch;
///
/// fn main() {
///     let mut buffer = [-1.0; 16];
///     Window::fill(Welch, &mut buffer);
///     assert_eq!(buffer, [
///         0.00000000000000000,
///         0.24888888888888883,
///         0.46222222222222210,
///         0.64000000000000000,
///         0.78222222222222220,
///         0.88888888888888880,
///         0.96000000000000000,
///         0.99555555555555550,
///         0.99555555555555550,
///         0.96000000000000000,
///         0.88888888888888900,
///         0.78222222222222240,
///         0.63999999999999990,
///         0.46222222222222210,
///         0.24888888888888883,
///         0.00000000000000000,
///     ]);
/// }
/// ```
pub struct Welch;

impl<F: Float> Window<F> for Welch {
    fn calc(&self, x: F) -> F {
        let i = F::from(2.0).unwrap() * x - F::one();
        F::one() - (i * i)
    }
}

/// Represents a Hann window.
///
/// ```
/// use sampara::window::Window;
/// use sampara::window::types::Hann;
///
/// fn main() {
///     let mut buffer = [-1.0; 16];
///     Window::fill(Hann, &mut buffer);
///     assert_eq!(buffer, [
///         0.00000000000000000,
///         0.04322727117869957,
///         0.16543469682057088,
///         0.34549150281252630,
///         0.55226423163382670,
///         0.74999999999999990,
///         0.90450849718747370,
///         0.98907380036690280,
///         0.98907380036690280,
///         0.90450849718747370,
///         0.75000000000000020,
///         0.55226423163382710,
///         0.34549150281252640,
///         0.16543469682057077,
///         0.04322727117869951,
///         0.00000000000000000,
///     ]);
/// }
/// ```
pub struct Hann;

impl<F: Float + FloatConst> Window<F> for Hann {
    fn calc(&self, x: F) -> F {
        (F::one() - (F::TAU() * x).cos()) * F::from(0.5).unwrap()
    }
}

/// Represents a Blackman window.
///
/// ```
/// use sampara::window::Window;
/// use sampara::window::types::Blackman;
///
/// fn main() {
///     let mut buffer = [-1.0; 16];
///     Window::fill(Blackman, &mut buffer);
///     assert_eq!(buffer, [
///         -0.000000000000000013877787807814457,
///         0.016757719687408210,
///         0.077072419759158600,
///         0.200770143262530450,
///         0.394012423575122200,
///         0.629999999999999900,
///         0.849229856737469400,
///         0.982157436978310800,
///         0.982157436978310800,
///         0.849229856737469500,
///         0.630000000000000200,
///         0.394012423575122670,
///         0.200770143262530560,
///         0.077072419759158530,
///         0.016757719687408162,
///         -0.000000000000000013877787807814457,
///     ]);
/// }
/// ```
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
