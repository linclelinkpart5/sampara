use crate::sample::Sample;

use crate::frame::{Frame, Iter, IterMut};

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Fixed<S: Sample, const N: usize>([S; N]);

impl<S: Sample, const N: usize> Fixed<S, N> {
    pub fn into_array(self) -> [S; N] {
        self.0
    }
}

impl<S: Sample, const N: usize> Default for Fixed<S, N> {
    fn default() -> Self {
        Self::EQUILIBRIUM
    }
}

impl<S: Sample, const N: usize> Frame for Fixed<S, N> {
    type Sample = S;

    const EQUILIBRIUM: Self = Fixed([S::EQUILIBRIUM; N]);

    fn get(&self, channel: usize) -> Option<&S> {
        self.0.get(channel)
    }

    fn get_mut(&mut self, channel: usize) -> Option<&mut S> {
        self.0.get_mut(channel)
    }

    fn iter(&self) -> Iter<'_, S> {
        Iter(self.0.iter())
    }

    fn iter_mut(&mut self) -> IterMut<'_, S> {
        IterMut(self.0.iter_mut())
    }

    fn len(&self) -> usize {
        N
    }
}

impl<S: Sample, const N: usize> IntoIterator for Fixed<S, N> {
    type Item = S;
    type IntoIter = IntoIter<S, N>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter())
    }
}

#[derive(Clone)]
pub struct IntoIter<S: Sample, const N: usize>(core::array::IntoIter<S, N>);

impl<S: Sample, const N: usize> Iterator for IntoIter<S, N> {
    type Item = S;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<S: Sample, const N: usize> ExactSizeIterator for IntoIter<S, N> {
    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<S: Sample, const N: usize> DoubleEndedIterator for IntoIter<S, N> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}
