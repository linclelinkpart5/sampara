use core::f64::consts::PI;

use crate::{Frame, Signal};

/// Types that can produce a phase step size, usually based on a target
/// frequency divided by a sampling frequency (sample rate).
///
/// These types are mainly used for driving oscillators and other periodic
/// [`Signal`]s, which advance one step at a time for each output.
pub trait Step<const N: usize> {
    type Step: Frame<N, Sample = f64>;

    fn step(&mut self) -> Option<Self::Step>;
}

pub struct ConstHz<F, const N: usize>
where
    F: Frame<N, Sample = f64>,
{
    step: F,
}

impl<F, const N: usize> ConstHz<F, N>
where
    F: Frame<N, Sample = f64>,
{
    pub fn new(rate: f64, hz: F) -> Self {
        let step = hz.apply(|x| x / rate);
        Self { step }
    }
}

pub struct VariableHz<S, const N: usize>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    hzs: S,
    rate: f64,
}

impl<S, const N: usize> VariableHz<S, N>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    pub fn new(rate: f64, hz_signal: S) -> Self {
        Self {
            hzs: hz_signal,
            rate,
        }
    }
}

impl<F, const N: usize> Step<N> for ConstHz<F, N>
where
    F: Frame<N, Sample = f64>,
{
    type Step = F;

    fn step(&mut self) -> Option<Self::Step> {
        Some(self.step)
    }
}

impl<S, const N: usize> Step<N> for VariableHz<S, N>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    type Step = S::Frame;

    fn step(&mut self) -> Option<Self::Step> {
        self.hzs.next().map(|f| f.mul_amp(1.0 / self.rate))
    }
}

/// A [`Signal`] that wraps a [`Step`] and accumulates it in an automated way,
/// wrapping it to the interval [0.0, 1.0) as needed.
///
/// ```
/// use sampara::generator::Phase;
/// use sampara::Signal;
///
/// fn main() {
///     let mut phase = Phase::const_hz(44100.0, 440.0);
///
///     assert_eq!(phase.next(), Some(0.009977324263038548));
///     assert_eq!(phase.next(), Some(0.019954648526077097));
///     assert_eq!(phase.next(), Some(0.029931972789115645));
///
///     // [`Phase`] keeps track of the accumutated steps, and resets back to
///     // 0.0 if it exceeds 1.0.
///     let mut phase = Phase::const_hz(1.1, 0.5);
///     assert_eq!(phase.next(), Some(0.45454545454545453));
///     assert_eq!(phase.next(), Some(0.9090909090909091));
///     assert_eq!(phase.next(), Some(0.36363636363636354));
/// }
/// ```
pub struct Phase<S, const N: usize>
where
    S: Step<N>,
{
    stepper: S,
    accum: S::Step,
}

impl<F, const N: usize> Phase<ConstHz<F, N>, N>
where
    F: Frame<N, Sample = f64>,
{
    /// Creates a [`Phase`] with a constant [`Frame`] of frequencies.
    ///
    /// This [`Phase`] does not terminate, it will always return a step value.
    ///
    /// ```
    /// use sampara::generator::Phase;
    /// use sampara::Signal;
    ///
    /// fn main() {
    ///     let mut phase = Phase::const_hz(4.0, [0.5, 1.0, 1.5]);
    ///
    ///     assert_eq!(phase.next(), Some([0.125, 0.25, 0.375]));
    ///     assert_eq!(phase.next(), Some([0.25, 0.5, 0.75]));
    ///     assert_eq!(phase.next(), Some([0.375, 0.75, 0.125]));
    /// }
    /// ```
    pub fn const_hz(rate: f64, hz: F) -> Self {
        Self {
            stepper: ConstHz::new(rate, hz),
            accum: Frame::EQUILIBRIUM,
        }
    }
}

impl<S, const N: usize> Phase<VariableHz<S, N>, N>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    /// Creates a [`Phase`] with [`Frame`]s of frequencies over time, as
    /// yielded by a [`Signal`].
    ///
    /// Unlike [`const_hz`], this [`Phase`] will terminate and stop yielding
    /// step values once the contained [`Signal`] is fully consumed.
    ///
    /// ```
    /// use sampara::generator::Phase;
    /// use sampara::{signal, Signal};
    ///
    /// fn main() {
    ///     let freq_signal = signal::from_frames(vec![
    ///         [0.125, 0.250],
    ///         [0.375, 0.500],
    ///         [0.625, 0.750],
    ///     ]);
    ///
    ///     let mut phase = Phase::variable_hz(4.0, freq_signal);
    ///
    ///     // Note that this [`Phase`] terminates once the contained [`Signal`]
    ///     // is consumed.
    ///     assert_eq!(phase.next(), Some([0.03125, 0.0625]));
    ///     assert_eq!(phase.next(), Some([0.125, 0.1875]));
    ///     assert_eq!(phase.next(), Some([0.28125, 0.375]));
    ///     assert_eq!(phase.next(), None);
    /// }
    /// ```
    pub fn variable_hz(rate: f64, hz_signal: S) -> Self {
        Self {
            stepper: VariableHz::new(rate, hz_signal),
            accum: Frame::EQUILIBRIUM,
        }
    }
}

impl<S, const N: usize> Signal<N> for Phase<S, N>
where
    S: Step<N>,
{
    type Frame = S::Step;

    fn next(&mut self) -> Option<Self::Frame> {
        let phase = self.accum
            .add_frame(self.stepper.step()?.into_signed_frame())
            .apply(|x| x % 1.0);

        self.accum = phase;
        Some(phase)
    }
}

/// A sine wave [`Signal`] generator.
pub struct Sine<S, const N: usize>
where
    S: Step<N>,
{
    phase: Phase<S, N>,
}

impl<S, const N: usize> Signal<N> for Sine<S, N>
where
    S: Step<N>,
{
    type Frame = S::Step;

    fn next(&mut self) -> Option<Self::Frame> {
        self.phase.next().map(|mut phase| {
            phase.transform(|p| (2.0 * PI * p).sin());
            phase
        })
    }
}

/// A saw wave [`Signal`] generator.
pub struct Saw<S, const N: usize>
where
    S: Step<N>,
{
    phase: Phase<S, N>,
}

impl<S, const N: usize> Signal<N> for Saw<S, N>
where
    S: Step<N>,
{
    type Frame = S::Step;

    fn next(&mut self) -> Option<Self::Frame> {
        self.phase.next().map(|mut phase| {
            phase.transform(|p| p * -2.0 + 1.0);
            phase
        })
    }
}

/// A square wave [`Signal`] generator.
pub struct Square<S, const N: usize>
where
    S: Step<N>,
{
    phase: Phase<S, N>,
}

impl<S, const N: usize> Signal<N> for Square<S, N>
where
    S: Step<N>,
{
    type Frame = S::Step;

    fn next(&mut self) -> Option<Self::Frame> {
        self.phase.next().map(|mut phase| {
            phase.transform(|p| {
                if p < 0.5 { 1.0 }
                else { -1.0 }
            });
            phase
        })
    }
}
