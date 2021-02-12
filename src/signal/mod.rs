mod adaptors;
mod generators;
mod iterators;

use crate::{Frame, Sample};
#[cfg(feature = "biquad")]
use crate::{Duplex, biquad::{Param, Params}, sample::FloatSample};

pub use adaptors::*;
pub use generators::*;
pub use iterators::*;

/// Types that yield a sequence of [`Frame`]s, representing an audio signal.
///
/// This trait is inspired by the [`Iterator`] trait and has similar methods
/// and adaptors, but with a DSP-related focus.
pub trait Signal<const N: usize> {
    /// The [`Frame`] type returned by this [`Signal`].
    type Frame: Frame<N>;

    /// Advances [`Self`] and returns the next [`Frame`], or [`None`] if there
    /// are no more to yield.
    fn next(&mut self) -> Option<Self::Frame>;

    /// Similar to [`next`], but will always yield a [`Frame`]. Yields
    /// [`Frame::EQUILIBRIUM`] if there are no more actual [`Frame`]s to yield.
    fn sig_next(&mut self) -> Self::Frame {
        self.next().unwrap_or(Frame::EQUILIBRIUM)
    }

    /// Borrows this [`Signal`] rather than consuming it.
    ///
    /// This is useful for applying adaptors while still retaining ownership of
    /// the original [`Signal`].
    ///
    /// ```rust
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

    /// Creates a new [`Signal`] that applies a function to each [`Frame`] of
    /// [`Self`].
    fn map<F, M, const NF: usize>(self, func: M) -> Map<Self, F, M, N, NF>
    where
        Self: Sized,
        F: Frame<NF>,
        M: FnMut(Self::Frame) -> F
    {
        Map {
            signal: self,
            func,
        }
    }

    /// Creates a new [`Signal`] that applies a function to each pair of
    /// [`Frame`]s in [`Self`] and another [`Signal`].
    fn zip_map<O, F, M, const NO: usize, const NF: usize>(
        self,
        other: O,
        func: M,
    ) -> ZipMap<Self, O, F, M, N, NO, NF>
    where
        Self: Sized,
        O: Signal<NO>,
        F: Frame<NF>,
        M: FnMut(Self::Frame, O::Frame) -> F
    {
        ZipMap {
            signal_a: self,
            signal_b: other,
            func,
        }
    }

    /// Creates a new [`Signal`] that yields the sum of pairs of [`Frame`]s
    /// yielded by [`Self`] and another [`Signal`] in lockstep.
    fn add_signal<B>(self, other: B) -> AddSignal<Self, B, N>
    where
        Self: Sized,
        B: Signal<N>,
        Self::Frame: Frame<N, Signed = <B::Frame as Frame<N>>::Signed>,
    {
        AddSignal {
            signal_a: self,
            signal_b: other,
        }
    }

    /// Creates a new [`Signal`] that yields the product of pairs of [`Frame`]s
    /// yielded by [`Self`] and another [`Signal`] in lockstep.
    fn mul_signal<B>(self, other: B) -> MulSignal<Self, B, N>
    where
        Self: Sized,
        B: Signal<N>,
        Self::Frame: Frame<N, Float = <B::Frame as Frame<N>>::Float>,
    {
        MulSignal {
            signal_a: self,
            signal_b: other,
        }
    }

    /// Creates a new [`Signal`] that yields each [`Frame`] of a [`Signal`]
    /// summed with a constant [`Frame`].
    fn add_frame<F>(self, frame: F) -> AddFrame<Self, F, N>
    where
        Self: Sized,
        Self::Frame: Frame<N, Signed = F>,
        F: Frame<N>,
    {
        AddFrame {
            signal: self,
            frame,
        }
    }

    /// Creates a new [`Signal`] that yields each [`Frame`] of a [`Signal`]
    /// multiplied with a constant [`Frame`].
    fn mul_frame<F>(self, frame: F) -> MulFrame<Self, F, N>
    where
        Self: Sized,
        Self::Frame: Frame<N, Float = F>,
        F: Frame<N>,
    {
        MulFrame {
            signal: self,
            frame,
        }
    }

    /// Creates a new [`Signal`] that yields each [`Frame`] of a [`Signal`]
    /// with each channel summed with a constant [`Sample`].
    fn add_amp<X>(self, amp: X) -> AddAmp<Self, X, N>
    where
        Self: Sized,
        Self::Frame: Frame<N>,
        <Self::Frame as Frame<N>>::Sample: Sample<Signed = X>,
        X: Sample,
    {
        AddAmp {
            signal: self,
            amp,
        }
    }

    /// Creates a new [`Signal`] that yields each [`Frame`] of a [`Signal`]
    /// with each channel multiplied with a constant [`Sample`].
    fn mul_amp<X>(self, amp: X) -> MulAmp<Self, X, N>
    where
        Self: Sized,
        Self::Frame: Frame<N>,
        <Self::Frame as Frame<N>>::Sample: Sample<Float = X>,
        X: Sample,
    {
        MulAmp {
            signal: self,
            amp,
        }
    }

    /// Delays [`Self`] by a given number of frames. The delay is performed by
    /// yielding [`Frame::EQUILIBRIUM`] that number of times before continuing
    /// to yield frames from [`Self`].
    fn delay(self, n_frames: usize) -> Delay<Self, N>
    where
        Self: Sized,
    {
        Delay {
            signal: self,
            n_frames,
        }
    }

    /// Calls an inspection function on each [`Frame`] yielded by this
    /// [`Signal`], and then passes the [`Frame`] through.
    ///
    /// ```rust
    /// use sampara::{signal, Signal};
    ///
    /// fn main() {
    ///     let mut max: Option<i32> = None;
    ///     let mut signal = signal::from_frames(vec![2i32, 3, 1])
    ///         .inspect(|&f| {
    ///             if let Some(m) = max {
    ///                 max.replace(m.max(f));
    ///             } else {
    ///                 max = Some(f);
    ///             }
    ///         });
    ///
    ///     assert_eq!(signal.next(), Some(2));
    ///     assert_eq!(signal.next(), Some(3));
    ///     assert_eq!(signal.next(), Some(1));
    ///     assert_eq!(signal.next(), None);
    ///     assert_eq!(max, Some(3));
    /// }
    /// ```
    fn inspect<F>(self, func: F) -> Inspect<Self, F, N>
    where
        Self: Sized,
        F: FnMut(&Self::Frame),
    {
        Inspect {
            signal: self,
            func,
        }
    }

    /// Performs biquad filtering on this [`Signal`] and yields filtered
    /// [`Frame`]s in the same format as the original [`Signal`].
    ///
    /// ```rust
    /// use sampara::{signal, Signal};
    /// use sampara::biquad::{Kind, Params};
    ///
    /// fn main() {
    ///     // Notch filter.
    ///     let params = Params::from_kind(Kind::Notch, 0.25, 0.7071);
    ///
    ///     let input_signal = signal::from_frames(vec![
    ///         [-57,  61], [ 50,  13], [  5,  91], [-16,  -7],
    ///         [ 74, -36], [ 85, -37], [-48,  19], [-64,  -8],
    ///         [  1,  77], [ 28,  45], [ 83,  47], [-34, -92],
    ///         [ 16,   4], [ 74,  45], [-89,   5], [-63, -53],
    ///     ]);
    ///
    ///     let expected = &[
    ///         [-33,  35], [ 29,   7], [-24,  82], [ 14,   2],
    ///         [ 50,  17], [ 37, -26], [  6, -13], [  5, -21],
    ///         [-28,  58], [-22,  25], [ 54,  62], [  0, -31],
    ///         [ 48,  19], [ 23, -22], [-51,   1], [  2,   0],
    ///     ];
    ///
    ///     let mut filtered_signal = input_signal.biquad(params);
    ///
    ///     let mut produced = vec![];
    ///     while let Some(filtered_frame) = filtered_signal.next() {
    ///         produced.push(filtered_frame);
    ///     }
    ///
    ///     assert_eq!(&produced, expected);
    /// }
    /// ```
    #[cfg(feature = "biquad")]
    fn biquad<P>(self, params: Params<P>) -> Biquad<Self, P, N>
    where
        Self: Sized,
        P: Param + FloatSample,
        <Self::Frame as Frame<N>>::Sample: Duplex<P>,
    {
        Biquad {
            signal: self,
            filter: params.into(),
        }
    }
}

impl<S, const N: usize> Signal<N> for &mut S
where
    S: Signal<N>,
{
    type Frame = S::Frame;

    fn next(&mut self) -> Option<Self::Frame> {
        (**self).next()
    }
}

/// Creates a new [`Signal`] where each [`Frame`] is yielded by calling a given
/// closure that produces a [`Option<Frame>`] for each iteration.
///
/// ```rust
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
pub fn from_fn<F, G, const N: usize>(gen_fn: G) -> FromFn<F, G, N>
where
    F: Frame<N>,
    G: FnMut() -> Option<F>,
{
    FromFn(gen_fn)
}

/// Creates a new [`Signal`] where each [`Frame`] is copied from a given
/// constant [`Frame`].
///
/// ```rust
/// use sampara::{signal, Signal};
///
/// fn main() {
///     let mut signal = signal::constant([1, 2, 3, 4]);
///
///     assert_eq!(signal.next(), Some([1, 2, 3, 4]));
///     assert_eq!(signal.next(), Some([1, 2, 3, 4]));
///     assert_eq!(signal.next(), Some([1, 2, 3, 4]));
///     assert_eq!(signal.next(), Some([1, 2, 3, 4]));
/// }
/// ```
pub fn constant<F, const N: usize>(frame: F) -> Constant<F, N>
where
    F: Frame<N>,
{
    Constant(frame)
}

/// Creates a new [`Signal`] that always yields [`Frame::EQUILIBRIUM`].
///
/// ```rust
/// use sampara::{signal, Signal};
///
/// fn main() {
///     let mut signal = signal::equilibrium();
///
///     assert_eq!(signal.next(), Some([0, 0]));
///     assert_eq!(signal.next(), Some([0, 0]));
///     assert_eq!(signal.next(), Some([0, 0]));
///     assert_eq!(signal.next(), Some([0, 0]));
/// }
/// ```
pub fn equilibrium<F, const N: usize>() -> Equilibrium<F, N>
where
    F: Frame<N>,
{
    Equilibrium(Default::default())
}

/// Creates an empty [`Signal`] that yields no [`Frame`]s.
///
/// ```rust
/// use sampara::{signal, Signal};
///
/// fn main() {
///     // Need to have redundant number of channels, until associated consts
///     // can be used as const generics.
///     let mut signal = signal::empty::<[i8; 2], 2>();
///
///     assert_eq!(signal.next(), None);
///     assert_eq!(signal.next(), None);
///     assert_eq!(signal.next(), None);
///     assert_eq!(signal.next(), None);
/// }
/// ```
pub fn empty<F, const N: usize>() -> Empty<F, N>
where
    F: Frame<N>,
{
    Empty(Default::default())
}

/// Creates a new [`Signal`] by wrapping an iterable that yields [`Frame`]s.
///
/// ```rust
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
pub fn from_frames<I, const N: usize>(iter: I) -> FromFrames<I::IntoIter, N>
where
    I: IntoIterator,
    I::Item: Frame<N>,
{
    FromFrames(iter.into_iter())
}

/// Creates a new [`Signal`] by wrapping an iterable that yields [`Samples`]s.
/// These [`Sample`]s are assumed to be interleaved, and in channel order.
/// The resulting [`Signal`] will read these [`Sample`]s into [`Frame`]s of the
/// desired size, and yield them. Any trailing [`Sample`]s that do not fully
/// complete a [`Frame`] will be discarded.
///
/// ```rust
/// use sampara::{signal, Signal};
///
/// fn main() {
///     let samples = vec![1, 2, 3, 4, 5, 6, 7];
///     let mut signal = signal::from_samples(samples);
///
///     assert_eq!(signal.next(), Some([1, 2]));
///     assert_eq!(signal.next(), Some([3, 4]));
///     assert_eq!(signal.next(), Some([5, 6]));
///     // Not enough remaining samples for a full frame, so they are discarded.
///     assert_eq!(signal.next(), None);
/// }
/// ```
pub fn from_samples<F, I, const N: usize>(iter: I) -> FromSamples<F, I::IntoIter, N>
where
    F: Frame<N, Sample = I::Item>,
    I: IntoIterator,
    I::Item: Sample,
{
    FromSamples(iter.into_iter(), Default::default())
}
