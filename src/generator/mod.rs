use core::f64::consts::PI;

use crate::{Frame, Signal};

/// Types that can produce a phase step size, usually based on a target
/// frequency divided by a sampling frequency (sample rate).
///
/// These types are mainly used for driving oscillators and other periodic
/// [`Signal`]s, which advance one step at a time for each output.
pub trait Delta<const N: usize>: Sized {
    type Delta: Frame<N, Sample = f64>;

    fn delta(&mut self) -> Option<Self::Delta>;

    fn phase(self) -> Phase<Self, N> {
        Phase {
            stepper: self,
            accum: Frame::EQUILIBRIUM,
        }
    }
}

pub struct Fixed<F, const N: usize>(F)
where
    F: Frame<N, Sample = f64>;

impl<F, const N: usize> Delta<N> for Fixed<F, N>
where
    F: Frame<N, Sample = f64>,
{
    type Delta = F;

    fn delta(&mut self) -> Option<Self::Delta> {
        Some(self.0)
    }
}

enum VarInner<S, const N: usize>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    Hzs(S, f64),
    Deltas(S),
}

impl<S, const N: usize> Delta<N> for VarInner<S, N>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    type Delta = S::Frame;

    fn delta(&mut self) -> Option<Self::Delta> {
        match self {
            Self::Hzs(hz_signal, rate) => hz_signal.next().map(|f| f.mul_amp(1.0 / *rate)),
            Self::Deltas(delta_signal) => delta_signal.next(),
        }
    }
}

pub struct Variable<S, const N: usize>(VarInner<S, N>)
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>;

impl<S, const N: usize> Delta<N> for Variable<S, N>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    type Delta = S::Frame;

    fn delta(&mut self) -> Option<Self::Delta> {
        self.0.delta()
    }
}

/// A [`Signal`] that wraps a [`Delta`] and accumulates it in an automated way,
/// wrapping it to the interval [0.0, 1.0) as needed.
///
/// ```
/// use sampara::generator;
/// use sampara::Signal;
///
/// fn main() {
///     let mut phase = generator::fixed_hz(44100.0, 440.0);
///
///     assert_eq!(phase.next(), Some(0.009977324263038548));
///     assert_eq!(phase.next(), Some(0.019954648526077097));
///     assert_eq!(phase.next(), Some(0.029931972789115645));
///
///     // [`Phase`] keeps track of the accumutated steps, and resets back to
///     // 0.0 if it exceeds 1.0.
///     let mut phase = generator::fixed_hz(1.1, 0.5);
///     assert_eq!(phase.next(), Some(0.45454545454545453));
///     assert_eq!(phase.next(), Some(0.9090909090909091));
///     assert_eq!(phase.next(), Some(0.36363636363636354));
/// }
/// ```
pub struct Phase<D, const N: usize>
where
    D: Delta<N>,
{
    stepper: D,
    accum: D::Delta,
}

impl<D, const N: usize> Signal<N> for Phase<D, N>
where
    D: Delta<N>,
{
    type Frame = D::Delta;

    fn next(&mut self) -> Option<Self::Frame> {
        let phase = self
            .accum
            .add_frame(self.stepper.delta()?.into_signed_frame())
            .map(|x| x % 1.0);

        self.accum = phase;
        Some(phase)
    }
}

/// Creates a [`Phase`] with a constant [`Frame`] of frequencies.
///
/// This [`Phase`] does not terminate, it will always return a step value.
///
/// ```
/// use sampara::generator;
/// use sampara::Signal;
///
/// fn main() {
///     let mut phase = generator::fixed_hz(4.0, [0.5, 1.0, 1.5]);
///
///     assert_eq!(phase.next(), Some([0.125, 0.25, 0.375]));
///     assert_eq!(phase.next(), Some([0.25, 0.5, 0.75]));
///     assert_eq!(phase.next(), Some([0.375, 0.75, 0.125]));
/// }
/// ```
pub fn fixed_hz<F, const N: usize>(rate: f64, hz: F) -> Phase<Fixed<F, N>, N>
where
    F: Frame<N, Sample = f64>,
{
    Fixed(hz.map(|x| x / rate)).phase()
}

/// Creates a [`Phase`] with a constant [`Frame`] of deltas.
///
/// This [`Phase`] does not terminate, it will always return a step value.
///
/// ```
/// use sampara::generator;
/// use sampara::Signal;
///
/// fn main() {
///     let mut phase = generator::fixed_step([0.125, 0.25, 0.375]);
///
///     assert_eq!(phase.next(), Some([0.125, 0.25, 0.375]));
///     assert_eq!(phase.next(), Some([0.25, 0.5, 0.75]));
///     assert_eq!(phase.next(), Some([0.375, 0.75, 0.125]));
/// }
/// ```
pub fn fixed_step<F, const N: usize>(delta: F) -> Phase<Fixed<F, N>, N>
where
    F: Frame<N, Sample = f64>,
{
    Fixed(delta).phase()
}

/// Creates a [`Phase`] with [`Frame`]s of deltas over time, as
/// yielded by a [`Signal`].
///
/// Unlike [`fixed_hz`], this [`Phase`] will terminate and stop yielding
/// step values once the contained [`Signal`] is fully consumed.
///
/// ```
/// use sampara::generator;
/// use sampara::{signal, Signal};
///
/// fn main() {
///     let freq_signal = signal::from_frames(vec![
///         [0.125, 0.250],
///         [0.375, 0.500],
///         [0.625, 0.750],
///     ]);
///
///     let mut phase = generator::variable_hz(4.0, freq_signal);
///
///     // Note that this [`Phase`] terminates once the contained [`Signal`]
///     // is consumed.
///     assert_eq!(phase.next(), Some([0.03125, 0.0625]));
///     assert_eq!(phase.next(), Some([0.125, 0.1875]));
///     assert_eq!(phase.next(), Some([0.28125, 0.375]));
///     assert_eq!(phase.next(), None);
/// }
/// ```
pub fn variable_hz<S, const N: usize>(rate: f64, hz_signal: S) -> Phase<Variable<S, N>, N>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    Variable(VarInner::Hzs(hz_signal, rate)).phase()
}

/// Creates a [`Phase`] with [`Frame`]s of deltas over time, as
/// yielded by a [`Signal`].
///
/// Unlike [`fixed_step`], this [`Phase`] will terminate and stop yielding
/// step values once the contained [`Signal`] is fully consumed.
///
/// ```
/// use sampara::generator;
/// use sampara::{signal, Signal};
///
/// fn main() {
///     let delta_signal = signal::from_frames(vec![
///         [0.03125, 0.0625],
///         [0.375, 0.500],
///         [0.625, 0.750],
///     ]);
///
///     let mut phase = generator::variable_step(delta_signal);
///
///     // Note that this [`Phase`] terminates once the contained [`Signal`]
///     // is consumed.
///     assert_eq!(phase.next(), Some([0.03125, 0.0625]));
///     assert_eq!(phase.next(), Some([0.40625, 0.5625]));
///     assert_eq!(phase.next(), Some([0.03125, 0.3125]));
///     assert_eq!(phase.next(), None);
/// }
/// ```
pub fn variable_step<S, const N: usize>(delta_signal: S) -> Phase<Variable<S, N>, N>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    Variable(VarInner::Deltas(delta_signal)).phase()
}

/// A sine wave [`Signal`] generator.
pub struct Sine<D, const N: usize>
where
    D: Delta<N>,
{
    phase: Phase<D, N>,
}

impl<D, const N: usize> Signal<N> for Sine<D, N>
where
    D: Delta<N>,
{
    type Frame = D::Delta;

    fn next(&mut self) -> Option<Self::Frame> {
        self.phase.next().map(|mut phase| {
            phase.transform(|p| (2.0 * PI * p).sin());
            phase
        })
    }
}

/// A saw wave [`Signal`] generator.
pub struct Saw<D, const N: usize>
where
    D: Delta<N>,
{
    phase: Phase<D, N>,
}

impl<D, const N: usize> Signal<N> for Saw<D, N>
where
    D: Delta<N>,
{
    type Frame = D::Delta;

    fn next(&mut self) -> Option<Self::Frame> {
        self.phase.next().map(|mut phase| {
            phase.transform(|p| p * -2.0 + 1.0);
            phase
        })
    }
}

/// A square wave [`Signal`] generator.
pub struct Square<D, const N: usize>
where
    D: Delta<N>,
{
    phase: Phase<D, N>,
}

impl<D, const N: usize> Signal<N> for Square<D, N>
where
    D: Delta<N>,
{
    type Frame = D::Delta;

    fn next(&mut self) -> Option<Self::Frame> {
        self.phase.next().map(|mut phase| {
            phase.transform(|p| if p < 0.5 { 1.0 } else { -1.0 });
            phase
        })
    }
}
