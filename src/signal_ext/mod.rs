mod adapters;

use crate::{Frame, Sample};

pub use adapters::*;

// NOTE: Example of a trait alias.
// trait Signal<const N: usize> = Iterator<Item: Frame<N>>;

pub trait Signal<const N: usize>: Iterator<Item = Self::Frame> {
    type Frame: Frame<N>;

    fn sig_next(&mut self) -> Self::Frame {
        self.next().unwrap_or(Frame::EQUILIBRIUM)
    }

    /// Creates a new [`Signal`] that applies a function to each pair of
    /// [`Frame`]s in [`Self`] and another [`Signal`].
    fn mix<I, Y, F, const NB: usize, const NY: usize>(self, other: I, func: F)
        -> Mix<Self, I::IntoIter, Y, F, N, NB, NY>
    where
        Self: Sized,
        I: IntoSignal<NB>,
        Y: Frame<NY>,
        F: FnMut(Self::Frame, <I::IntoIter as Signal<NB>>::Frame) -> Y,
    {
        Mix {
            signal_a: self,
            signal_b: other.into_iter(),
            func,
        }
    }
}

impl<I: ?Sized, const N: usize> Signal<N> for I
where
    I: Iterator<Item: Frame<N>>,
{
    type Frame = I::Item;
}

pub trait IntoSignal<const N: usize>: IntoIterator<IntoIter: Signal<N>> {
    type Frame;
    type Signal: Signal<N, Frame = Self::Frame>;
}

impl<I: ?Sized, const N: usize> IntoSignal<N> for I
where
    I: IntoIterator<Item: Frame<N>>,
{
    type Frame = I::Item;
    type Signal = I::IntoIter;
}
