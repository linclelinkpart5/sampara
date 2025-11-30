use crate::frame::Frame;

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