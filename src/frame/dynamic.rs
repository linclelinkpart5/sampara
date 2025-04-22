use std::alloc::Global;
use std::ptr::NonNull;

use crate::sample::Sample;

use crate::frame::{Frame, Iter, IterMut};

#[derive(Clone, Debug, PartialEq)]
pub struct Dynamic<S: Sample>(Box<[S]>);

impl<S: Sample> Dynamic<S> {
    pub fn into_boxed_slice(self) -> Box<[S]> {
        self.0
    }

    pub fn resize(&mut self, new_len: usize, s: S) {
        self.resize_with(new_len, || s);
    }

    pub fn resize_with<F>(&mut self, new_len: usize, f: F)
    where
        F: FnMut() -> S,
    {
        if self.len() != new_len {
            let mut contents: Box<[S]> = Box::new([]);
            core::mem::swap(&mut contents, &mut self.0);

            let mut v = Vec::from(contents);

            v.resize_with(new_len, f);

            let mut contents: Box<[S]> = v.into_boxed_slice();
            core::mem::swap(&mut contents, &mut self.0);
        }
    }

    pub fn truncate<F>(&mut self, new_len: usize)
    where
        F: FnMut() -> S,
    {
        if self.len() > new_len {
            let mut contents: Box<[S]> = Box::new([]);
            core::mem::swap(&mut contents, &mut self.0);

            let mut v = Vec::from(contents);

            v.truncate(new_len);

            let mut contents: Box<[S]> = v.into_boxed_slice();
            core::mem::swap(&mut contents, &mut self.0);
        }
    }
}

impl<S: Sample> Default for Dynamic<S> {
    fn default() -> Self {
        Self(Box::new([]))
    }
}

impl<S: Sample> From<Vec<S>> for Dynamic<S> {
    fn from(value: Vec<S>) -> Self {
        Self(value.into_boxed_slice())
    }
}

impl<S: Sample> From<Box<[S]>> for Dynamic<S> {
    fn from(value: Box<[S]>) -> Self {
        Self(value)
    }
}

impl<S: Sample, const N: usize> From<[S; N]> for Dynamic<S> {
    fn from(value: [S; N]) -> Self {
        Self(value.to_vec().into_boxed_slice())
    }
}

impl<S: Sample> Frame for Dynamic<S> {
    type Sample = S;

    fn equil() -> Self {
        Self(Box::new([]))
    }

    fn get(&self, channel: usize) -> Option<&S> {
        self.0.get(channel)
    }

    fn get_mut(&mut self, channel: usize) -> Option<&mut S> {
        self.0.get_mut(channel)
    }

    fn iter(&self) -> impl Iterator<Item = &Self::Sample> {
        Iter(self.0.iter())
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut Self::Sample> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize() {
        let mut f: Dynamic<i8> = Dynamic::from([-30, -10, 10, 30]);

        let contents = f.iter().copied().collect::<Vec<_>>();
        assert_eq!(contents, &[-30, -10, 10, 30]);

        f.resize(2, -128);

        let contents = f.iter().copied().collect::<Vec<_>>();
        assert_eq!(contents, &[-30, -10]);

        let mut f: Dynamic<i8> = Dynamic::from([-30, -10, 10, 30]);

        f.resize(8, -128);

        let contents = f.iter().copied().collect::<Vec<_>>();
        assert_eq!(contents, &[-30, -10, 10, 30, -128, -128, -128, -128]);
    }

    #[test]
    fn empty_identites() {
        let fs: Vec<Dynamic<i8>> = vec![
            Dynamic::equil(),
            Dynamic::default(),
            Dynamic::from(vec![]),
            Dynamic::from(vec![].into_boxed_slice()),
            Dynamic::from([]),
        ];

        let xs = fs.iter();
        let ys = fs.iter();
        for (x, y) in ys.flat_map(|y| xs.clone().map(move |x| (x, y))) {
            assert_eq!(x, y);
        }

        let mut f: Dynamic<i8> = Dynamic::equil();

        f.resize(1, 0);

        assert_eq!(Dynamic::<i8>::equil().len(), 0);
        assert_eq!(f.len(), 1);

        println!("{:?}", Dynamic::<i8>::equil());
    }
}
