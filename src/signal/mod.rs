mod adaptors;

use crate::{Frame, Sample};

pub use adaptors::*;

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
        self.next().unwrap_or(<Self::Frame as Frame<N>>::EQUILIBRIUM)
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
}
