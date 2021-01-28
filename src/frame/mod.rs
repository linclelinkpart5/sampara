use crate::sample::Sample;

/// A trait for working generically across `N`-sized blocks of [`Sample`]s,
/// representing sampling values across `N` channels at a single point in time.
/// Each of these blocks is called a "frame".
pub trait Frame<const N: usize>: Copy + Clone + PartialEq {
    /// The [`Sample`] type stored in each channel within the frame.
    type Sample: Sample;

    /// The equilibrium value for this [`Frame`] type.
    ///
    /// ```rust
    /// use sampara::{Frame, Mono, Stereo};
    ///
    /// fn main() {
    ///     assert_eq!(Mono::<f32>::EQUILIBRIUM, [0.0]);
    ///     assert_eq!(Stereo::<f32>::EQUILIBRIUM, [0.0, 0.0]);
    ///     assert_eq!(<[f32; 3]>::EQUILIBRIUM, [0.0, 0.0, 0.0]);
    ///     assert_eq!(<[u8; 2]>::EQUILIBRIUM, [128u8, 128]);
    /// }
    /// ```
    const EQUILIBRIUM: Self;

    /// Create a new [`Frame`] where `Sample` for each channel is produced by
    /// repeatedly calling the provided function.
    ///
    /// The function should map each channel index to a [`Sample`] value.
    ///
    /// ```rust
    /// use sampara::{Frame, Stereo};
    ///
    /// fn main() {
    ///     let frame = <[i8; 3]>::from_fn(|i| (i as i8 + 1) * 32);
    ///     assert_eq!(frame, [32, 64, 96]);
    ///
    ///     let frame = Stereo::<f32>::from_fn(|i| i as f32 * 0.5);
    ///     assert_eq!(frame, [0.0, 0.5]);
    /// }
    fn from_fn<F>(func: F) -> Self
    where
        F: FnMut(usize) -> Self::Sample;

    /// Create a new [`Frame`] from a borrowed [`Iterator`] yielding samples
    /// for each channel.
    ///
    /// Returns [`None`] if the given [`Iterator`] does not yield enough
    /// [`Sample`] values.
    ///
    /// ```rust
    /// use sampara::{Frame, Stereo};
    ///
    /// fn main() {
    ///     let mut samples = (0..=6).into_iter();
    ///
    ///     let opt_frame = Stereo::<_>::from_samples(&mut samples);
    ///     assert_eq!(opt_frame, Some([0, 1]));
    ///
    ///     let opt_frame = <[i8; 4]>::from_samples(&mut samples);
    ///     assert_eq!(opt_frame, Some([2, 3, 4, 5]));
    ///
    ///     let opt_frame = <[i8; 3]>::from_samples(&mut samples);
    ///     assert_eq!(opt_frame, None);
    /// }
    fn from_samples<I>(samples: &mut I) -> Option<Self>
    where
        I: Iterator<Item = Self::Sample>;
}

impl<S, const N: usize> Frame<N> for [S; N]
where
    S: Sample,
{
    type Sample = S;

    const EQUILIBRIUM: Self = [S::EQUILIBRIUM; N];

    fn from_fn<F>(mut func: F) -> Self
    where
        F: FnMut(usize) -> Self::Sample
    {
        let mut out = Self::EQUILIBRIUM;

        for (i, ch) in out.iter_mut().enumerate() {
            *ch = func(i);
        }

        out
    }

    fn from_samples<I>(samples: &mut I) -> Option<Self>
    where
        I: Iterator<Item = Self::Sample>
    {
        let mut out = Self::EQUILIBRIUM;

        for ch in out.iter_mut() {
            *ch = samples.next()?;
        }

        Some(out)
    }
}

impl<S> Frame<1> for S
where
    S: Sample,
{
    type Sample = S;

    const EQUILIBRIUM: Self = S::EQUILIBRIUM;

    fn from_fn<F>(mut func: F) -> Self
    where
        F: FnMut(usize) -> Self::Sample
    {
        func(0)
    }

    fn from_samples<I>(samples: &mut I) -> Option<Self>
    where
        I: Iterator<Item = Self::Sample>
    {
        samples.next()
    }
}

pub type Mono<S> = [S; 1];
pub type Stereo<S> = [S; 2];
