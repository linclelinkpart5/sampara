use crate::sample::Sample;

use crate::frame::{Frame, Iter, IterMut};

#[derive(Clone, Debug, PartialEq)]
pub struct Dynamic<S: Sample>(Box<[S]>);

impl<S: Sample> Dynamic<S> {
    pub fn into_boxed_slice(self) -> Box<[S]> {
        self.0
    }
}

impl<S: Sample> Default for Dynamic<S> {
    fn default() -> Self {
        Self(Box::new([]))
    }
}

impl<S: Sample> Frame<S> for Dynamic<S> {
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
        self.0.len()
    }
}

impl<S: Sample> IntoIterator for Dynamic<S> {
    type Item = S;
    type IntoIter = IntoIter<S>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter())
    }
}

#[derive(Clone)]
pub struct IntoIter<S: Sample>(std::vec::IntoIter<S>);

impl<S: Sample> Iterator for IntoIter<S> {
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

impl<S: Sample> ExactSizeIterator for IntoIter<S> {
    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<S: Sample> DoubleEndedIterator for IntoIter<S> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}
