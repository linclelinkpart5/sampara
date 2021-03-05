//! Contains [`Iterator`]s that are created from or are related to [`Signal`]s.

use crate::signal::Signal;

/// Converts a [`Signal`] into a [`Iterator`].
#[derive(Clone)]
pub struct IntoIter<S, const N: usize>
where
    S: Signal<N>,
{
    pub(super) signal: S,
}

impl<S, const N: usize> Iterator for IntoIter<S, N>
where
    S: Signal<N>,
{
    type Item = S::Frame;

    fn next(&mut self) -> Option<Self::Item> {
        self.signal.next()
    }
}
