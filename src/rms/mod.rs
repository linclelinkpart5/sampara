use num_traits::Float;

use crate::{Frame, Sample, Processor, Signal};
use crate::buffer::{Fixed, Buffer};
use crate::sample::FloatSample;
use crate::signal::Process;

const DO_SQRT: bool = true;
const NO_SQRT: bool = false;
const DO_POW2: bool = true;
const NO_POW2: bool = false;

struct StatsInner<F, B, const N: usize, const SQRT: bool, const POW2: bool>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    window: Fixed<B>,
    sum: F,
}

impl<F, B, const N: usize, const SQRT: bool, const POW2: bool> StatsInner<F, B, N, SQRT, POW2>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    #[inline]
    fn from_full(buffer: B) -> Self {
        let mut buffer = buffer;
        let mut sum = F::EQUILIBRIUM;

        for frame in buffer.as_mut().iter_mut() {
            if POW2 {
                // Since the passed-in buffer has raw frames, square them
                // in-place.
                frame.transform(|x| x * x);
            }

            sum.add_assign_frame(frame.into_signed_frame());
        }

        Self {
            window: Fixed::from(buffer),
            sum,
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.window.capacity()
    }

    #[inline]
    fn reset(&mut self) {
        // ASSUME: All float samples have an equilibrium of 0. That way this
        // code as written works for any combo of (SQRT, POW2).
        self.window.fill(Frame::EQUILIBRIUM);
        self.sum = Frame::EQUILIBRIUM;
    }

    #[inline]
    fn fill(&mut self, fill_val: F) {
        let mut fill_val = fill_val;

        if POW2 {
            // Calculate the squared frame, as that is what will actually be
            // stored in the window.
            fill_val.transform(|x| x * x);
        }

        self.window.fill(fill_val);

        // Since the buffer is filled with a constant value, just multiply to
        // calculate the sum.
        let len_f: F::Sample = Sample::from_sample(self.len() as f32);
        self.sum = fill_val.mul_amp(len_f);
    }

    #[inline]
    fn fill_with<M>(&mut self, fill_func: M)
    where
        M: FnMut() -> F,
    {
        let mut fill_func = fill_func;
        let mut sum = F::EQUILIBRIUM;

        let prepped_fill_func = || {
            let mut f = fill_func();

            if POW2 {
                // Square the frame.
                f.transform(|x| x * x);
            }

            // Before yielding the squared frame, add it to the running sum.
            sum.add_assign_frame(f.into_signed_frame());

            f
        };

        self.window.fill_with(prepped_fill_func);
        self.sum = sum;
    }

    #[inline]
    fn advance(&mut self, input: F) {
        let mut input = input;

        if POW2 {
            // Calculate the square of the new frame and push onto the buffer.
            input.transform(|x| x * x);
        }

        let popped = self.window.push(input);

        // Add the new input and subtract the popped frame from the sum.
        self.sum
            .add_assign_frame(input.into_signed_frame())
            .sub_assign_frame(popped.into_signed_frame());

        if SQRT {
            // In case of floating point rounding errors, floor at equilibrium.
            self.sum.transform(|x| x.max(Sample::EQUILIBRIUM));
        }
    }

    #[inline]
    fn current(&self) -> F {
        let len_f = Sample::from_sample(self.len() as f32);
        let mut ret: F = self.sum.apply(|s| s / len_f);

        if SQRT {
            ret.transform(Float::sqrt);
        }

        ret
    }

    #[inline]
    fn process(&mut self, input: F) -> F {
        self.advance(input);
        self.current()
    }
}

impl<F, B, const N: usize, const SQRT: bool, const POW2: bool> From<B> for StatsInner<F, B, N, SQRT, POW2>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    #[inline]
    fn from(buffer: B) -> Self {
        let mut new = Self {
            window: Fixed::from(buffer),
            sum: Frame::EQUILIBRIUM,
        };

        new.reset();

        new
    }
}

impl<F, B, const N: usize, const SQRT: bool, const POW2: bool> Processor<N, N> for StatsInner<F, B, N, SQRT, POW2>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    type Input = F;
    type Output = F;

    #[inline]
    fn process(&mut self, input: Self::Input) -> Self::Output {
        self.process(input)
    }
}

/// Keeps a running MS (mean square) of a window of [`Frame`]s over time.
#[derive(Clone)]
pub struct Ms<F, B, const N: usize>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    window: Fixed<B>,
    square_sum: F,
}

impl<F, B, const N: usize> Ms<F, B, N>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    /// Similar to [`Ms::from`], but treats the passed-in buffer as already
    /// filled with input [`Frame`]s.
    ///
    /// ```
    /// use sampara::rms::Ms;
    ///
    /// fn main() {
    ///     let mut ms = Ms::from_full([[0.5], [0.5], [0.5], [0.5]]);
    ///     assert_eq!(ms.current(), [0.25]);
    ///
    ///     assert_eq!(ms.process([1.0]), [0.4375]);
    ///     assert_eq!(ms.process([1.0]), [0.6250]);
    ///     assert_eq!(ms.process([1.0]), [0.8125]);
    ///     assert_eq!(ms.process([1.0]), [1.0]);
    /// }
    /// ```
    #[inline]
    pub fn from_full(buffer: B) -> Self {
        let mut buffer = buffer;
        let mut square_sum = F::EQUILIBRIUM;

        // Since the passed-in buffer has raw frames, square them inplace and
        // calculate the square sum.
        for frame in buffer.as_mut().iter_mut() {
            frame.transform(|x| x * x);

            // TODO: See if `zip_transform` can make this more efficient.
            square_sum = square_sum.add_frame(frame.into_signed_frame());
        }

        Self {
            window: Fixed::from(buffer),
            square_sum,
        }
    }

    /// Resets the MS window to its zeroed-out state.
    ///
    /// ```
    /// use sampara::rms::Ms;
    ///
    /// fn main() {
    ///     let mut ms = Ms::from_full([[0.25], [0.75], [-0.25], [-0.75]]);
    ///     assert_ne!(ms.current(), [0.0]);
    ///
    ///     ms.reset();
    ///     assert_eq!(ms.current(), [0.0]);
    /// }
    /// ```
    #[inline]
    pub fn reset(&mut self) {
        self.window.fill(Frame::EQUILIBRIUM);
        self.square_sum = Frame::EQUILIBRIUM;
    }

    /// Fills the MS window with a single constant [`Frame`] value.
    ///
    /// ```
    /// use sampara::rms::Ms;
    ///
    /// fn main() {
    ///     let mut ms = Ms::from([[0.0]; 4]);
    ///
    ///     ms.fill([0.5]);
    ///     assert_eq!(ms.current(), [0.25]);
    ///
    ///     ms.advance([1.0]);
    ///     ms.advance([1.0]);
    ///     assert_eq!(ms.current(), [0.625]);
    /// }
    /// ```
    #[inline]
    pub fn fill(&mut self, fill: F) {
        let mut fill = fill;

        // Calculate the squared frame, as that is what will actually be stored
        // in the window.
        fill.transform(|x| x * x);

        self.window.fill(fill);

        let num_frames_f: F::Sample = Sample::from_sample(self.len() as f32);
        self.square_sum = fill.apply(|x| num_frames_f * x);
    }

    /// Fills the MS window by repeatedly calling a closure that produces
    /// [`Frame`] values.
    ///
    /// ```
    /// use sampara::rms::Ms;
    ///
    /// fn main() {
    ///     let mut ms = Ms::from([[0.0]; 4]);
    ///
    ///     let mut zero = true;
    ///     ms.fill_with(|| {
    ///         zero = !zero;
    ///         if zero { [0.0] }
    ///         else { [1.0] }
    ///     });
    ///     assert_eq!(ms.current(), [0.5]);
    ///
    ///     ms.advance([1.0]);
    ///     ms.advance([1.0]);
    ///     assert_eq!(ms.current(), [0.75]);
    /// }
    /// ```
    #[inline]
    pub fn fill_with<M>(&mut self, func: M)
    where
        M: FnMut() -> F,
    {
        let mut func = func;
        let mut sq_sum = F::EQUILIBRIUM;

        let sq_func = || {
            let mut f = func();

            // Square the frame.
            f.transform(|x| x * x);

            // Before yielding the squared frame, add it to the running square
            // sum.
            sq_sum = sq_sum.add_frame(f.into_signed_frame());

            f
        };

        self.window.fill_with(sq_func);
        self.square_sum = sq_sum;
    }

    /// Returns the length of the MS window buffer.
    ///
    /// ```
    /// use sampara::rms::Ms;
    ///
    /// fn main() {
    ///     const LEN: usize = 99;
    ///     let ms = Ms::from([[0.0; 2]; LEN]);
    ///     assert_eq!(ms.len(), LEN);
    /// }
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.window.capacity()
    }

    /// Advances the state of the MS window buffer by pushing in a new input
    /// [`Frame`]. The oldest frame will be popped off in order to accomodate
    /// the new one.
    ///
    /// This method does not calculate the current MS value, so it is useful
    /// for workflows that process multiple frames in bulk and then calculate
    /// the MS value afterwards.
    ///
    /// ```
    /// use sampara::rms::Ms;
    ///
    /// fn main() {
    ///     let mut ms = Ms::from([[0.0; 2]; 4]);
    ///     assert_eq!(ms.current(), [0.0, 0.0]);
    ///
    ///     ms.advance([1.0, 1.0]);
    ///     ms.advance([1.0, 1.0]);
    ///     assert_eq!(ms.current(), [0.5, 0.5]);
    /// }
    /// ```
    #[inline]
    pub fn advance(&mut self, input: F) {
        // Calculate the square of the new frame and push onto the buffer.
        let input_sq = input.apply(|s| s * s);
        let popped_sq_frame = self.window.push(input_sq);

        // Add the new frame square and subtract the removed frame square.
        self.square_sum =
            self.square_sum
                .add_frame(input_sq.into_signed_frame())
                .zip_apply(popped_sq_frame, |s, r| {
                    // In case of floating point rounding errors, floor at
                    // equilibrium.
                    (s - r).max(Sample::EQUILIBRIUM)
                });
    }

    /// Calculates the MS value using the current window contents.
    ///
    /// ```
    /// use sampara::rms::Ms;
    ///
    /// fn main() {
    ///     let mut ms = Ms::from_full([[0.0], [1.0], [0.0], [1.0]]);
    ///     assert_eq!(ms.current(), [0.5]);
    /// }
    /// ```
    #[inline]
    pub fn current(&self) -> F {
        let num_frames_f = Sample::from_sample(self.len() as f32);
        self.square_sum.apply(|s| s / num_frames_f)
    }

    /// Processes a new input frame by advancing the state of the MS window
    /// buffer and then calculating the current MS value.
    ///
    /// This is equivalent to a call to [`Self::advance`] followed by a call to
    /// [`Self::current`].
    ///
    /// ```
    /// use sampara::rms::Ms;
    ///
    /// fn main() {
    ///     let mut ms = Ms::from([[0.0]; 4]);
    ///     assert_eq!(ms.process([1.0]), [0.25]);
    ///     assert_eq!(ms.process([-1.0]), [0.5]);
    ///     assert_eq!(ms.process([1.0]), [0.75]);
    ///     assert_eq!(ms.process([-1.0]), [1.0]);
    /// }
    /// ```
    #[inline]
    pub fn process(&mut self, input: F) -> F {
        self.advance(input);
        self.current()
    }
}

impl<F, B, const N: usize> From<B> for Ms<F, B, N>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    /// Creates a new [`Ms`] using a given [`Buffer`] as a window.
    ///
    /// The contents of the buffer will be discarded and overwritten with
    /// equilibrium values.
    ///
    /// ```
    /// use sampara::rms::Ms;
    ///
    /// fn main() {
    ///     // These values get zeroed out.
    ///     let mut ms = Ms::from([[-1.0]; 4]);
    ///     assert_eq!(ms.current(), [0.0]);
    ///
    ///     assert_eq!(ms.process([1.0]), [0.25]);
    ///     assert_eq!(ms.process([1.0]), [0.5]);
    ///     assert_eq!(ms.process([1.0]), [0.75]);
    ///     assert_eq!(ms.process([1.0]), [1.0]);
    /// }
    /// ```
    #[inline]
    fn from(buffer: B) -> Self {
        let mut new = Self {
            window: Fixed::from(buffer),
            square_sum: Frame::EQUILIBRIUM,
        };

        new.reset();

        new
    }
}

impl<F, B, const N: usize> Processor<N, N> for Ms<F, B, N>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    type Input = F;
    type Output = F;

    #[inline]
    fn process(&mut self, input: Self::Input) -> Self::Output {
        self.process(input)
    }
}

/// Keeps a running RMS (root mean square) of a window of [`Frame`]s over time.
pub struct Rms<F, B, const N: usize>(Ms<F, B, N>)
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
;

impl<F, B, const N: usize> Rms<F, B, N>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    /// Similar to [`Rms::from`], but treats the passed-in buffer as already
    /// filled with input [`Frame`]s.
    ///
    /// ```
    /// use sampara::rms::Rms;
    ///
    /// fn main() {
    ///     let mut rms = Rms::from_full([[0.5], [0.5], [0.5], [0.5]]);
    ///     assert_eq!(rms.current(), [0.5]);
    ///
    ///     assert_eq!(rms.process([1.0]), [0.6614378277661477]);
    ///     assert_eq!(rms.process([1.0]), [0.7905694150420949]);
    ///     assert_eq!(rms.process([1.0]), [0.9013878188659973]);
    ///     assert_eq!(rms.process([1.0]), [1.0]);
    /// }
    /// ```
    #[inline]
    pub fn from_full(buffer: B) -> Self {
        Self(Ms::from_full(buffer))
    }

    /// Resets the RMS window to its zeroed-out state.
    ///
    /// ```
    /// use sampara::rms::Rms;
    ///
    /// fn main() {
    ///     let mut rms = Rms::from_full([[0.25], [0.75], [-0.25], [-0.75]]);
    ///     assert_ne!(rms.current(), [0.0]);
    ///
    ///     rms.reset();
    ///     assert_eq!(rms.current(), [0.0]);
    /// }
    /// ```
    #[inline]
    pub fn reset(&mut self) {
        self.0.reset()
    }

    /// Fills the RMS window with a single constant [`Frame`] value.
    ///
    /// ```
    /// use sampara::rms::Rms;
    ///
    /// fn main() {
    ///     let mut rms = Rms::from([[0.0]; 4]);
    ///
    ///     rms.fill([0.5]);
    ///     assert_eq!(rms.current(), [0.5]);
    ///
    ///     rms.advance([1.0]);
    ///     rms.advance([1.0]);
    ///     assert_eq!(rms.current(), [0.7905694150420949]);
    /// }
    /// ```
    #[inline]
    pub fn fill(&mut self, fill: F) {
        self.0.fill(fill)
    }

    /// Fills the RMS window by repeatedly calling a closure that produces
    /// [`Frame`] values.
    ///
    /// ```
    /// use sampara::rms::Rms;
    ///
    /// fn main() {
    ///     let mut rms = Rms::from([[0.0]; 4]);
    ///
    ///     let mut zero = true;
    ///     rms.fill_with(|| {
    ///         zero = !zero;
    ///         if zero { [0.0] }
    ///         else { [1.0] }
    ///     });
    ///     assert_eq!(rms.current(), [0.7071067811865476]);
    ///
    ///     rms.advance([1.0]);
    ///     rms.advance([1.0]);
    ///     assert_eq!(rms.current(), [0.8660254037844386]);
    /// }
    /// ```
    #[inline]
    pub fn fill_with<M>(&mut self, func: M)
    where
        M: FnMut() -> F,
    {
        self.0.fill_with(func)
    }

    /// Returns the length of the RMS window buffer.
    ///
    /// ```
    /// use sampara::rms::Rms;
    ///
    /// fn main() {
    ///     const LEN: usize = 99;
    ///     let rms = Rms::from([[0.0; 2]; LEN]);
    ///     assert_eq!(rms.len(), LEN);
    /// }
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Advances the state of the RMS window buffer by pushing in a new input
    /// [`Frame`]. The oldest frame will be popped off in order to accomodate
    /// the new one.
    ///
    /// This method does not calculate the current RMS value, so it is useful
    /// for workflows that process multiple frames in bulk and then calculate
    /// the RMS value afterwards.
    ///
    /// ```
    /// use sampara::rms::Rms;
    ///
    /// fn main() {
    ///     let mut rms = Rms::from([[0.0; 2]; 4]);
    ///     assert_eq!(rms.current(), [0.0, 0.0]);
    ///
    ///     rms.advance([1.0, 1.0]);
    ///     rms.advance([1.0, 1.0]);
    ///     assert_eq!(rms.current(), [0.7071067811865476, 0.7071067811865476]);
    /// }
    /// ```
    #[inline]
    pub fn advance(&mut self, input: F) {
        self.0.advance(input)
    }

    /// Calculates the RMS value using the current window contents.
    ///
    /// ```
    /// use sampara::rms::Rms;
    ///
    /// fn main() {
    ///     let mut rms = Rms::from_full([[0.0], [1.0], [0.0], [1.0]]);
    ///     assert_eq!(rms.current(), [0.7071067811865476]);
    /// }
    /// ```
    #[inline]
    pub fn current(&self) -> F {
        self.0.current().apply(Float::sqrt)
    }

    /// Processes a new input frame by advancing the state of the RMS window
    /// buffer and then calculating the current RMS value.
    ///
    /// This is equivalent to a call to [`Self::advance`] followed by a call to
    /// [`Self::current`].
    ///
    /// ```
    /// use sampara::rms::Rms;
    ///
    /// fn main() {
    ///     let mut rms = Rms::from([[0.0]; 4]);
    ///     assert_eq!(rms.process([1.0]), [0.5]);
    ///     assert_eq!(rms.process([-1.0]), [0.7071067811865476]);
    ///     assert_eq!(rms.process([1.0]), [0.8660254037844386]);
    ///     assert_eq!(rms.process([-1.0]), [1.0]);
    /// }
    /// ```
    #[inline]
    pub fn process(&mut self, input: F) -> F {
        self.0.process(input).apply(Float::sqrt)
    }
}

impl<F, B, const N: usize> From<B> for Rms<F, B, N>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    /// Creates a new [`Rms`] using a given [`Buffer`] as a window.
    ///
    /// The contents of the buffer will be discarded and overwritten with
    /// equilibrium values.
    ///
    /// ```
    /// use sampara::rms::Rms;
    ///
    /// fn main() {
    ///     // These values get zeroed out.
    ///     let mut rms = Rms::from([[-1.0]; 4]);
    ///     assert_eq!(rms.current(), [0.0]);
    ///
    ///     assert_eq!(rms.process([1.0]), [0.5]);
    ///     assert_eq!(rms.process([1.0]), [0.7071067811865476]);
    ///     assert_eq!(rms.process([1.0]), [0.8660254037844386]);
    ///     assert_eq!(rms.process([1.0]), [1.0]);
    /// }
    /// ```
    #[inline]
    fn from(buffer: B) -> Self {
        Self(Ms::from(buffer))
    }
}

impl<F, B, const N: usize> Processor<N, N> for Rms<F, B, N>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    type Input = F;
    type Output = F;

    #[inline]
    fn process(&mut self, input: Self::Input) -> Self::Output {
        self.process(input)
    }
}

impl<S, B, const N: usize> Process<S, Ms<S::Frame, B, N>, N, N>
where
    S: Signal<N>,
    <S::Frame as Frame<N>>::Sample: FloatSample,
    B: Buffer<Item = S::Frame>,
{
    #[inline]
    pub fn current(&self) -> S::Frame {
        self.processor.current()
    }
}

impl<S, B, const N: usize> Process<S, Rms<S::Frame, B, N>, N, N>
where
    S: Signal<N>,
    <S::Frame as Frame<N>>::Sample: FloatSample,
    B: Buffer<Item = S::Frame>,
{
    #[inline]
    pub fn current(&self) -> S::Frame {
        self.processor.current()
    }
}
