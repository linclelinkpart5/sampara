mod dynamic;
mod fixed;

pub use self::dynamic::Dynamic;
pub use self::fixed::Fixed;

use core::fmt::Debug;

use crate::sample::Sample;

pub trait Frame: Clone + PartialEq + Debug + Default + IntoIterator<Item = Self::Sample> {
    type Sample: Sample;

    const EQUILIBRIUM: Self;

    fn get(&self, channel: usize) -> Option<&Self::Sample>;

    fn get_mut(&mut self, channel: usize) -> Option<&mut Self::Sample>;

    fn iter(&self) -> Iter<'_, Self::Sample>;

    fn iter_mut(&mut self) -> IterMut<'_, Self::Sample>;

    fn len(&self) -> usize;
}

/// An iterator that yields the [`Sample`] for each channel in the frame by
/// reference.
#[derive(Clone)]
pub struct Iter<'a, S: Sample>(core::slice::Iter<'a, S>);

impl<'a, S: Sample> Iterator for Iter<'a, S> {
    type Item = &'a S;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<'a, S: Sample> ExactSizeIterator for Iter<'a, S> {
    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a, S: Sample> DoubleEndedIterator for Iter<'a, S> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

/// Like [`Iter`], but yields mutable references instead.
pub struct IterMut<'a, S: Sample>(core::slice::IterMut<'a, S>);

impl<'a, S: Sample> Iterator for IterMut<'a, S> {
    type Item = &'a mut S;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<'a, S: Sample> ExactSizeIterator for IterMut<'a, S> {
    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a, S: Sample> DoubleEndedIterator for IterMut<'a, S> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}
