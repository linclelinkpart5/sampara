mod adapters;
mod sources;

use crate::{Sample, frame::Frame, signal::sources::{FromFn, FromFrames, FromSamplesDynamic, FromSamplesFixed}};

/// Types that yield a sequence of [`Frame`]s, representing an audio signal.
///
/// This trait is inspired by the [`Iterator`] trait and has similar methods
/// and adaptors, but with a DSP-related focus.
pub trait Signal {
    /// The [`Frame`] type returned by this [`Signal`].
    type Frame: Frame;

    /// Advances [`Self`] and returns the next [`Frame`], or [`None`] if there
    /// are no more to yield.
    fn next(&mut self) -> Option<Self::Frame>;

    /// Similar to [`Self::next`], but will always yield a [`Frame`]. Yields
    /// [`Frame::EQUILIBRIUM`] if there are no more actual [`Frame`]s to yield.
    fn sig_next(&mut self) -> Self::Frame {
        self.next().unwrap_or(Self::Frame::equil())
    }

    /// Returns the `n`th [`Frame`] from this [`Signal`], starting at 0. This
    /// will advance the [`Signal`], so multiple calls to `nth` with the same
    /// `n` will give different results.
    ///
    /// ```
    /// use sampara::{signal, Signal};
    ///
    /// fn main() {
    ///     let mut signal = signal::from_frames(0..=9);
    ///
    ///     assert_eq!(signal.nth(3), Some(3));
    ///     assert_eq!(signal.nth(3), Some(7));
    ///     assert_eq!(signal.nth(3), None);
    /// }
    /// ```
    fn nth(&mut self, n: usize) -> Option<Self::Frame> {
        self.advance_by(n).ok()?;
        self.next()
    }

    /// Borrows this [`Signal`] rather than consuming it.
    ///
    /// This is useful for applying adaptors while still retaining ownership of
    /// the original [`Signal`].
    ///
    /// ```
    /// use sampara::{signal, Signal};
    ///
    /// fn main() {
    ///     let mut signal = signal::from_frames(vec![0, 1, 2, 3]);
    ///     assert_eq!(signal.next(), Some(0));
    ///     assert_eq!(signal.by_ref().add_amp(10).next(), Some(11));
    ///     assert_eq!(signal.by_ref().mul_amp(2.5_f32).next(), Some(5));
    ///     assert_eq!(signal.next(), Some(3));
    /// }
    /// ```
    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }

    /// Eagerly advances and discards `N` [`Frame`]s from [`Self`]. If there
    /// are fewer than `N` [`Frame`]s found, this will return `Err(X)`, where
    /// `X` is the number of [`Frame`]s actually advanced. Otherwise, returns
    /// `Ok(())`.
    ///
    /// ```
    /// use sampara::{signal, Signal};
    ///
    /// fn main() {
    ///     let mut signal = signal::from_frames(0u8..=9);
    ///
    ///     // Skip ahead 5 frames.
    ///     assert_eq!(signal.advance_by(5), Ok(()));
    ///
    ///     assert_eq!(signal.next(), Some(5));
    ///     assert_eq!(signal.next(), Some(6));
    ///
    ///     // Try to skip ahead 5 more frames.
    ///     assert_eq!(signal.advance_by(5), Err(3));
    ///
    ///     assert_eq!(signal.next(), None);
    /// }
    /// ```
    fn advance_by(&mut self, n: usize) -> Result<(), usize> {
        let mut left = n;
        while left > 0 {
            self.next().ok_or_else(|| n - left)?;
            left -= 1;
        }

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////
/*                            MODULE-LEVEL_METHODS                            */
////////////////////////////////////////////////////////////////////////////////

/// Creates a new [`Signal`] where each [`Frame`] is yielded by calling a given
/// closure that produces a [`Option<Frame>`] for each iteration.
///
/// ```
/// use sampara::{signal, Signal};
///
/// fn main() {
///     let mut state = 1;
///     let mut signal = signal::from_fn(|| {
///         if state < 4 {
///             let frame = [state, state * 2, state * 3];
///             state += 1;
///             Some(frame)
///         }
///         else { None }
///     });
///
///     assert_eq!(signal.next(), Some([1, 2, 3]));
///     assert_eq!(signal.next(), Some([2, 4, 6]));
///     assert_eq!(signal.next(), Some([3, 6, 9]));
///     assert_eq!(signal.next(), None);
/// }
/// ```
pub fn from_fn<F, G>(gen_fn: G) -> FromFn<F, G>
where
    F: Frame,
    G: FnMut() -> Option<F>,
{
    FromFn(gen_fn)
}

// /// Creates a new [`Signal`] where each [`Frame`] is copied from a given
// /// constant [`Frame`].
// ///
// /// ```
// /// use sampara::{signal, Signal};
// ///
// /// fn main() {
// ///     let mut signal = signal::constant([1, 2, 3, 4]);
// ///
// ///     assert_eq!(signal.next(), Some([1, 2, 3, 4]));
// ///     assert_eq!(signal.next(), Some([1, 2, 3, 4]));
// ///     assert_eq!(signal.next(), Some([1, 2, 3, 4]));
// ///     assert_eq!(signal.next(), Some([1, 2, 3, 4]));
// /// }
// /// ```
// pub fn constant<F, const N: usize>(frame: F) -> Constant<F, N>
// where
//     F: Frame<N>,
// {
//     Constant(frame)
// }

// /// Creates a new [`Signal`] that always yields [`Frame::EQUILIBRIUM`].
// ///
// /// ```
// /// use sampara::{signal, Signal};
// ///
// /// fn main() {
// ///     let mut signal = signal::equilibrium();
// ///
// ///     assert_eq!(signal.next(), Some([0, 0]));
// ///     assert_eq!(signal.next(), Some([0, 0]));
// ///     assert_eq!(signal.next(), Some([0, 0]));
// ///     assert_eq!(signal.next(), Some([0, 0]));
// /// }
// /// ```
// pub fn equilibrium<F, const N: usize>() -> Equilibrium<F, N>
// where
//     F: Frame<N>,
// {
//     Equilibrium(Default::default())
// }

// /// Creates an empty [`Signal`] that yields no [`Frame`]s.
// ///
// /// ```
// /// use sampara::{signal, Signal};
// ///
// /// fn main() {
// ///     // Need to have redundant number of channels, until associated consts
// ///     // can be used as const generics.
// ///     let mut signal = signal::empty::<[i8; 2], 2>();
// ///
// ///     assert_eq!(signal.next(), None);
// ///     assert_eq!(signal.next(), None);
// ///     assert_eq!(signal.next(), None);
// ///     assert_eq!(signal.next(), None);
// /// }
// /// ```
// pub fn empty<F, const N: usize>() -> Empty<F, N>
// where
//     F: Frame<N>,
// {
//     Empty(Default::default())
// }

/// Creates a new [`Signal`] by wrapping an iterable that yields [`Frame`]s.
///
/// ```
/// use sampara::{signal, Signal};
///
/// fn main() {
///     let frames = vec![[0, 0], [16, -16], [32, -32]];
///     let mut signal = signal::from_frames(frames);
///
///     assert_eq!(signal.next(), Some([0, 0]));
///     assert_eq!(signal.next(), Some([16, -16]));
///     assert_eq!(signal.next(), Some([32, -32]));
///     assert_eq!(signal.next(), None);
/// }
/// ```
pub fn from_frames<I>(iter: I) -> FromFrames<I::IntoIter>
where
    I: IntoIterator,
    I::Item: Frame,
{
    FromFrames(iter.into_iter())
}

/// Creates a new [`Signal`] by wrapping an iterable that yields [`Sample`]s.
/// These [`Sample`]s are assumed to be interleaved, and in channel order.
/// The resulting [`Signal`] will read these [`Sample`]s into [`Frame`]s of the
/// desired size, and yield them. Any trailing [`Sample`]s that do not fully
/// complete a [`Frame`] will be discarded.
///
/// ```
/// use sampara::{signal, Signal};
/// use sampara::frame::Fixed;
///
/// fn main() {
///     let samples = vec![1, 2, 3, 4, 5, 6, 7];
///     let mut signal = signal::from_samples_fixed(samples);
///
///     assert_eq!(signal.next(), Some([1, 2].into()));
///     assert_eq!(signal.next(), Some([3, 4].into()));
///     assert_eq!(signal.next(), Some([5, 6].into()));
///     // Not enough remaining samples for a full frame, so they are discarded.
///     assert_eq!(signal.next(), None);
/// }
/// ```
pub fn from_samples_fixed<I, const N: usize>(iter: I) -> FromSamplesFixed<I::IntoIter, N>
where
    I: IntoIterator,
    I::Item: Sample,
{
    FromSamplesFixed(iter.into_iter())
}

pub fn from_samples_dynamic<I>(iter: I, n: usize) -> FromSamplesDynamic<I::IntoIter>
where
    I: IntoIterator,
    I::Item: Sample,
{
    FromSamplesDynamic(iter.into_iter(), n)
}