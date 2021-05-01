use num_traits::Float;

use crate::{Frame, Sample, sample::FloatSample};
use crate::buffer::{Fixed, Buffer};

/// Keeps a running RMS (root mean square) of a window of [`Frame`]s over time.
#[derive(Clone)]
pub struct Rms<F, B, const N: usize>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    window: Fixed<B>,
    square_sum: F,
}

impl<F, B, const N: usize> Rms<F, B, N>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    /// Creates a new [`Rms`] using a given [`Buffer`] as a window.
    /// The initial contents of the [`Buffer`] will be overwritten with
    /// equilibrium values.
    ///
    /// ```
    /// use sampara::rms::Rms;
    ///
    /// fn main() {
    ///     let mut rms = Rms::new([[0.0]; 4]);
    ///     rms.next([0.5]);
    /// }
    /// ```
    #[inline]
    pub fn new(buffer: B) -> Self {
        let mut new = Self {
            window: Fixed::from(buffer),
            square_sum: Frame::EQUILIBRIUM,
        };

        new.reset();

        new
    }

    /// Similar to [`new`], but treats the passed-in buffer as already filled.
    ///
    /// ```
    /// use sampara::rms::Rms;
    ///
    /// fn main() {
    ///     let rms = Rms::from_full([[0.00], [0.25], [0.50], [0.75]]);
    ///     assert_eq!(
    ///         rms.into_window().into_buffer(),
    ///         [[0.0], [0.0625], [0.25], [0.5625]],
    ///     );
    /// }
    /// ```
    pub fn from_full(buffer: B) -> Self {
        let mut buffer = buffer;
        let mut square_sum: F = Frame::EQUILIBRIUM;

        // Since the passed-in buffer has raw frames, square them inplace and
        // calculate the square sum.
        for frame in buffer.as_mut().iter_mut() {
            frame.transform(|x| x * x);
            square_sum = square_sum.add_frame(frame.into_signed_frame());
        }

        Self {
            window: Fixed::from(buffer),
            square_sum,
        }
    }

    /// Resets [`Self`] to its zeroed-out state.
    ///
    /// ```
    /// use sampara::rms::Rms;
    ///
    /// fn main() {
    ///     let mut rms = Rms::new([[1.0], [2.0], [3.0], [4.0]]);
    ///     rms.reset();
    ///     assert_eq!(
    ///         rms.into_window().into_buffer(),
    ///         [[0.0], [0.0], [0.0], [0.0]],
    ///     );
    /// }
    /// ```
    #[inline]
    pub fn reset(&mut self) {
        self.window.fill(Frame::EQUILIBRIUM);
        self.square_sum = Frame::EQUILIBRIUM;
    }

    /// Returns the window size of [`Self`].
    ///
    /// ```
    /// use sampara::rms::Rms;
    ///
    /// fn main() {
    ///     const LEN: usize = 99;
    ///     let rms = Rms::new([[0.0; 2]; LEN]);
    ///     assert_eq!(rms.len(), LEN);
    /// }
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.window.capacity()
    }

    /// Adds a new [`Frame`] to the buffer and returns the RMS of the new
    /// window's contents.
    ///
    /// The oldest [`Frame`] will be popped off, and the new one added.
    ///
    /// ```
    /// use sampara::rms::Rms;
    ///
    /// fn main() {
    ///     let mut rms = Rms::new([[0.0]; 4]);
    ///     assert_eq!(rms.next([1.0]), [0.5]);
    ///     assert_eq!(rms.next([-1.0]), [0.7071067811865476]);
    ///     assert_eq!(rms.next([1.0]), [0.8660254037844386]);
    ///     assert_eq!(rms.next([-1.0]), [1.0]);
    /// }
    /// ```
    // TODO: Should this accept any compatible frame, or should it only accept
    //       float frames?
    #[inline]
    pub fn next<I>(&mut self, new_frame: I) -> F
    where
        I: Frame<N, Float = F>,
    {
        self.next_squared(new_frame).apply(Float::sqrt)
    }

    /// Similar to [`next`], but skips the final square root calculation,
    /// yielding the MS (mean square) as opposed to the RMS (root mean square).
    ///
    /// ```
    /// use sampara::rms::Rms;
    ///
    /// fn main() {
    ///     let mut rms = Rms::new([[0.0]; 4]);
    ///     assert_eq!(rms.next_squared([1.0]), [0.25]);
    ///     assert_eq!(rms.next_squared([-1.0]), [0.5]);
    ///     assert_eq!(rms.next_squared([1.0]), [0.75]);
    ///     assert_eq!(rms.next_squared([-1.0]), [1.0]);
    /// }
    /// ```
    #[inline]
    pub fn next_squared<I>(&mut self, new_frame: I) -> F
    where
        I: Frame<N, Float = F>,
    {
        // Calculate the square of the new frame and push onto the buffer.
        let new_frame_square = new_frame.into_float_frame().apply(|s| s * s);
        let removed_frame_square = self.window.push(new_frame_square);

        // Add the new frame square and subtract the removed frame square.
        self.square_sum =
            self.square_sum
                .add_frame(new_frame_square.into_signed_frame())
                .zip_apply(removed_frame_square, |s, r| {
                    // In case of floating point rounding errors, floor at
                    // equilibrium.
                    (s - r).max(Sample::EQUILIBRIUM)
                });

        self.calc_rms_squared()
    }

    /// Returns the RMS of the current contents of the buffer.
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
        self.calc_rms_squared().apply(Float::sqrt)
    }

    /// Similar to [`current`], but skips the final square root calculation,
    /// yielding the MS (mean square) as opposed to the RMS (root mean square).
    ///
    /// ```
    /// use sampara::rms::Rms;
    ///
    /// fn main() {
    ///     let mut rms = Rms::from_full([[0.0], [1.0], [0.0], [1.0]]);
    ///     assert_eq!(rms.current_squared(), [0.5]);
    /// }
    /// ```
    #[inline]
    pub fn current_squared(&self) -> F {
        self.calc_rms_squared()
    }

    #[inline]
    fn calc_rms_squared(&self) -> F {
        let num_frames_f = Sample::from_sample(self.len() as f32);
        self.square_sum.apply(|s| s / num_frames_f)
    }

    /// Returns a reference to the underlying window.
    ///
    /// ```
    /// use sampara::rms::Rms;
    ///
    /// fn main() {
    ///     let rms = Rms::from_full([0.0, 1.0, 2.0]);
    ///     let window = rms.window().iter().copied().collect::<Vec<_>>();
    ///     assert_eq!(window, &[0.0, 1.0, 4.0]);
    /// }
    /// ```
    #[inline]
    pub fn window(&self) -> &Fixed<B> {
        &self.window
    }

    /// Consumes [`Self`] and yields the underlying window.
    ///
    /// ```
    /// use sampara::rms::Rms;
    ///
    /// fn main() {
    ///     let rms = Rms::from_full([0.0, 1.0, 2.0]);
    ///     let window = rms.into_window().iter().copied().collect::<Vec<_>>();
    ///     assert_eq!(window, &[0.0, 1.0, 4.0]);
    /// }
    /// ```
    #[inline]
    pub fn into_window(self) -> Fixed<B> {
        self.window
    }
}
