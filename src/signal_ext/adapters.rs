use crate::{Frame, Sample};
use crate::signal_ext::Signal;
#[cfg(feature = "biquad")]
use crate::{Duplex, biquad::{Param, Filter as BQFilter}, sample::FloatSample};

#[inline]
fn mix_helper<A, B, Y, F, const NA: usize, const NB: usize, const NY: usize>(
    signal_a: &mut A,
    signal_b: &mut B,
    mut func: F,
) -> Option<Y>
where
    A: Signal<NA>,
    B: Signal<NB>,
    F: FnMut(A::Frame, B::Frame) -> Y,
    Y: Frame<NY>,
{
    Some(func(signal_a.next()?, signal_b.next()?))
}

/// Maps a function to each pair of [`Frame`]s from two [`Signal`]s in lockstep
/// and yields a new [`Frame`].
#[derive(Clone)]
pub struct Mix<A, B, Y, F, const NA: usize, const NB: usize, const NY: usize>
where
    A: Signal<NA>,
    B: Signal<NB>,
    F: FnMut(A::Frame, B::Frame) -> Y,
    Y: Frame<NY>,
{
    pub(super) signal_a: A,
    pub(super) signal_b: B,
    pub(super) func: F,
}

impl<A, B, F, Y, const NA: usize, const NB: usize, const NY: usize> Iterator
for Mix<A, B, Y, F, NA, NB, NY>
where
    A: Signal<NA>,
    B: Signal<NB>,
    F: FnMut(A::Frame, B::Frame) -> Y,
    Y: Frame<NY>,
{
    type Item = Y;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        mix_helper(&mut self.signal_a, &mut self.signal_b, &mut self.func)
    }
}
