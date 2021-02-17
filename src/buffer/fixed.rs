use std::iter::FusedIterator;
use std::slice::{Iter as SliceIter, IterMut as SliceIterMut};

use crate::buffer::Storage;

/// A ring buffer (also known as a circular/cyclic buffer) with a fixed capacity
/// and FIFO semantics.
///
/// A [`Fixed`] ring buffer is always considered to be full, which means that:
/// * Popping an element off requires a new element to be pushed in.
/// * The length of the buffer is always equal to the capacity.
/// * The initial data used to create the buffer is considered to be active,
/// valid data.
///
/// The elements contained must be [`Copy`], due to the way elements are handled
/// during pushing and popping.
///
/// A [`Fixed`] ring buffer can be created out of any type that be coerced to
/// both immutable and mutable slices (i.e. implements both `AsRef<[T]>` and
/// `AsMut<[T]>`). Examples of such types include (but are not limited to):
/// * `&mut [T]`,
/// * `[T; N]` (for any `N`)
/// * `&mut [T; N]` (for any `N`)
/// * `Vec<T>`
///
/// ```rust
/// use sampara::buffer::Fixed;
///
/// fn main() {
///     // From a mutable slice.
///     let mut data = vec![0, 1, 2];
///     let buffer = Fixed::from(data.as_mut_slice());
///
///     // From an array.
///     let buffer = Fixed::from([3, 4, 5]);
///
///     // From a mutable array reference.
///     let mut data = [6, 7, 8];
///     let buffer = Fixed::from(&mut data);
///
///     // From a `Vec`.
///     let buffer = Fixed::from(vec![9, 10, 11]);
/// }
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Fixed<S>
where
    S: Storage,
{
    head: usize,
    storage: S,
}

impl<S> Fixed<S>
where
    S: Storage,
{
    /// Sets all values of this buffer to a given value, and sets the head
    /// index to 0.
    ///
    /// ```rust
    /// use sampara::buffer::Fixed;
    ///
    /// fn main() {
    ///     let mut buffer = Fixed::from([1, 2, 3]);
    ///     buffer.fill(0);
    ///     assert_eq!(buffer.push(4), 0);
    ///     assert_eq!(buffer.push(5), 0);
    ///     assert_eq!(buffer.push(6), 0);
    ///     assert_eq!(buffer.push(7), 4);
    /// }
    /// ```
    pub fn fill(&mut self, item: S::Item) {
        for e in self.storage.as_mut().iter_mut() {
            *e = item;
        }

        self.head = 0;
    }

    /// Sets all values of this buffer using a given closure, and sets the head
    /// index to 0.
    ///
    /// ```rust
    /// use sampara::buffer::Fixed;
    ///
    /// fn main() {
    ///     let mut buffer = Fixed::from([0, 0, 0]);
    ///     let mut counter = 0;
    ///     buffer.fill_with(|| {
    ///         counter += 11;
    ///         counter
    ///     });
    ///     assert_eq!(buffer.push(4), 11);
    ///     assert_eq!(buffer.push(5), 22);
    ///     assert_eq!(buffer.push(6), 33);
    ///     assert_eq!(buffer.push(7), 4);
    /// }
    /// ```
    pub fn fill_with<F>(&mut self, mut func: F)
    where
        F: FnMut() -> S::Item,
    {
        for e in self.storage.as_mut().iter_mut() {
            *e = func();
        }

        self.head = 0;
    }

    /// Returns the maximum number of elements this buffer can contain.
    ///
    /// ```rust
    /// use sampara::buffer::Fixed;
    ///
    /// fn main() {
    ///     let buffer = Fixed::from([1, 2, 3, 4]);
    ///     assert_eq!(buffer.capacity(), 4);
    /// }
    /// ```
    #[inline]
    pub fn capacity(&self) -> usize {
        self.storage.as_ref().len()
    }

    /// Pushes a new element onto the rear of the buffer, and pops off and
    /// returns the replaced element from the front.
    ///
    /// ```rust
    /// use sampara::buffer::Fixed;
    ///
    /// fn main() {
    ///     let mut buffer = Fixed::from([1, 2, 3]);
    ///     assert_eq!(buffer.push(4), 1);
    ///     assert_eq!(buffer.push(5), 2);
    ///     assert_eq!(buffer.push(6), 3);
    ///     assert_eq!(buffer.push(7), 4);
    ///     assert_eq!(buffer.push(8), 5);
    ///     assert_eq!(buffer.push(9), 6);
    /// }
    /// ```
    pub fn push(&mut self, item: S::Item) -> S::Item {
        if self.capacity() == 0 {
            // Storage has zero capacity, just re-return the passed-in element.
            return item;
        }

        let mut next_head = self.head + 1;
        if next_head >= self.capacity() {
            next_head = 0;
        }

        // Bounds checking can be skipped safely since the length is constant.
        let old_item = unsafe {
            std::mem::replace(self.storage.as_mut().get_unchecked_mut(self.head), item)
        };
        self.head = next_head;
        old_item
    }

    fn wrapped(&self, index: usize) -> usize {
        (self.head + index) % self.capacity()
    }

    /// Returns a reference to the element at the given index.
    ///
    /// If the index is out of range it will be looped around the length of the
    /// buffer.
    ///
    /// ```rust
    /// use sampara::buffer::Fixed;
    ///
    /// fn main() {
    ///     let buffer = Fixed::from([0, 1, 2]);
    ///     assert_eq!(buffer.get(0), &0);
    ///     assert_eq!(buffer.get(1), &1);
    ///     assert_eq!(buffer.get(2), &2);
    ///     assert_eq!(buffer.get(3), &0);
    ///     assert_eq!(buffer.get(4), &1);
    ///     assert_eq!(buffer.get(5), &2);
    /// }
    /// ```
    #[inline]
    pub fn get(&self, index: usize) -> &S::Item {
        let wrapped_index = self.wrapped(index);
        &self.storage.as_ref()[wrapped_index]
    }

    /// Similar to [`Fixed::get`], but returns a mutable reference instead.
    ///
    /// ```rust
    /// use sampara::buffer::Fixed;
    ///
    /// fn main() {
    ///     let mut buffer = Fixed::from([0, 1, 2]);
    ///     assert_eq!(buffer.get_mut(0), &mut 0);
    ///     assert_eq!(buffer.get_mut(1), &mut 1);
    ///     assert_eq!(buffer.get_mut(2), &mut 2);
    ///     assert_eq!(buffer.get_mut(3), &mut 0);
    ///     assert_eq!(buffer.get_mut(4), &mut 1);
    ///     assert_eq!(buffer.get_mut(5), &mut 2);
    /// }
    /// ```
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> &mut S::Item {
        let wrapped_index = self.wrapped(index);
        &mut self.storage.as_mut()[wrapped_index]
    }

    /// Constructs a [`Fixed`] ring buffer from a given inner storage and
    /// starting index.
    ///
    /// This method should only be used if you require specifying a first index.
    /// For most use cases, it is better to use [`Fixed::from`] instead.
    ///
    /// ```rust
    /// use sampara::buffer::Fixed;
    ///
    /// fn main() {
    ///     let mut buffer = Fixed::from_raw_parts(1, [0, 1, 2]);
    ///     assert_eq!(buffer.push(7), 1);
    ///     assert_eq!(buffer.push(8), 2);
    ///     assert_eq!(buffer.push(9), 0);
    ///
    ///     // Equivalent to the above.
    ///     let mut buffer = Fixed::from_raw_parts(7, [0, 1, 2]);
    ///     assert_eq!(buffer.push(7), 1);
    ///     assert_eq!(buffer.push(8), 2);
    ///     assert_eq!(buffer.push(9), 0);
    /// }
    /// ```
    #[inline]
    pub fn from_raw_parts(head: usize, storage: S) -> Self {
        let wrapped_head = head.checked_rem(storage.as_ref().len()).unwrap_or(0);

        Self {
            head: wrapped_head,
            storage,
        }
    }

    /// Decomposes a [`Fixed`] ring buffer into a head index and inner storage.
    ///
    /// ```rust
    /// use sampara::buffer::Fixed;
    ///
    /// fn main() {
    ///     let mut buffer = Fixed::from([0, 1, 2]);
    ///     buffer.push(6);
    ///     buffer.push(7);
    ///
    ///     assert_eq!(buffer.into_raw_parts(), (2, [6, 7, 2]));
    /// }
    /// ```
    #[inline]
    pub fn into_raw_parts(self) -> (usize, S) {
        let Self { head, storage } = self;
        (head, storage)
    }

    /// Decomposes a [`Fixed`] ring buffer into an inner storage.
    ///
    /// ```rust
    /// use sampara::buffer::Fixed;
    ///
    /// fn main() {
    ///     let mut buffer = Fixed::from([0, 1, 2]);
    ///     buffer.push(6);
    ///     buffer.push(7);
    ///
    ///     assert_eq!(buffer.into_inner(), [6, 7, 2]);
    /// }
    /// ```
    #[inline]
    pub fn into_inner(self) -> S {
        let (_, storage) = self.into_raw_parts();
        storage
    }

    fn as_slices(&self) -> (&[S::Item], &[S::Item]) {
        let (tail, head) = self.storage.as_ref().split_at(self.head);
        (head, tail)
    }

    fn as_slices_mut(&mut self) -> (&mut [S::Item], &mut [S::Item]) {
        let (tail, head) = self.storage.as_mut().split_at_mut(self.head);
        (head, tail)
    }

    pub fn iter(&self) -> Iter<'_, S::Item> {
        let (head, tail) = self.as_slices();

        Iter {
            head: head.iter(),
            tail: tail.iter(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, S::Item> {
        let (head, tail) = self.as_slices_mut();

        IterMut {
            head: head.iter_mut(),
            tail: tail.iter_mut(),
        }
    }
}

impl<S> From<S> for Fixed<S>
where
    S: Storage,
{
    /// Constructs a [`Fixed`] ring buffer from a given inner storage.
    ///
    /// ```rust
    /// use sampara::buffer::Fixed;
    ///
    /// fn main() {
    ///     let mut buffer = Fixed::from([0, 1, 2]);
    ///     assert_eq!(buffer.push(7), 0);
    ///     assert_eq!(buffer.push(8), 1);
    ///     assert_eq!(buffer.push(9), 2);
    /// }
    /// ```
    fn from(storage: S) -> Self {
        Self::from_raw_parts(0, storage)
    }
}

impl<S> AsRef<[S::Item]> for Fixed<S>
where
    S: Storage,
{
    fn as_ref(&self) -> &[S::Item] {
        self.storage.as_ref()
    }
}

impl<S> AsMut<[S::Item]> for Fixed<S>
where
    S: Storage,
{
    fn as_mut(&mut self) -> &mut [S::Item] {
        self.storage.as_mut()
    }
}

pub struct Iter<'a, I> {
    head: SliceIter<'a, I>,
    tail: SliceIter<'a, I>,
}

impl<'a, I> Iterator for Iter<'a, I> {
    type Item = &'a I;

    fn next(&mut self) -> Option<Self::Item> {
        self.head.next().or_else(|| self.tail.next())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<'a, I> ExactSizeIterator for Iter<'a, I> {
    fn len(&self) -> usize {
        self.head.len() + self.tail.len()
    }
}

impl<'a, I> DoubleEndedIterator for Iter<'a, I> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.tail.next_back().or_else(|| self.head.next_back())
    }
}

impl<'a, I> FusedIterator for Iter<'a, I> {}

pub struct IterMut<'a, I> {
    head: SliceIterMut<'a, I>,
    tail: SliceIterMut<'a, I>,
}

impl<'a, I> Iterator for IterMut<'a, I> {
    type Item = &'a mut I;

    fn next(&mut self) -> Option<Self::Item> {
        self.head.next().or_else(|| self.tail.next())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<'a, I> ExactSizeIterator for IterMut<'a, I> {
    fn len(&self) -> usize {
        self.head.len() + self.tail.len()
    }
}

impl<'a, I> DoubleEndedIterator for IterMut<'a, I> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.tail.next_back().or_else(|| self.head.next_back())
    }
}

impl<'a, I> FusedIterator for IterMut<'a, I> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iter() {
        let head = [1, 2, 3];
        let tail = [4, 5, 6];

        let iter = Iter {
            head: head.iter(),
            tail: tail.iter(),
        };

        assert_eq!(iter.collect::<Vec<_>>(), vec![&1, &2, &3, &4, &5, &6]);

        let iter_rev = Iter {
            head: head.iter(),
            tail: tail.iter(),
        }.rev();

        assert_eq!(iter_rev.collect::<Vec<_>>(), vec![&6, &5, &4, &3, &2, &1]);

        let mut iter = Iter {
            head: head.iter(),
            tail: tail.iter(),
        };

        assert_eq!(iter.len(), 6);
        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.next_back(), Some(&6));
        assert_eq!(iter.len(), 4);
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next_back(), Some(&5));
        assert_eq!(iter.len(), 2);
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next_back(), Some(&4));
        assert_eq!(iter.len(), 0);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
    }

    #[test]
    fn iter_mut() {
        let mut head = [1, 2, 3];
        let mut tail = [4, 5, 6];

        let iter = IterMut {
            head: head.iter_mut(),
            tail: tail.iter_mut(),
        };

        assert_eq!(iter.collect::<Vec<_>>(), vec![&mut 1, &mut 2, &mut 3, &mut 4, &mut 5, &mut 6]);

        let iter_rev = IterMut {
            head: head.iter_mut(),
            tail: tail.iter_mut(),
        }.rev();

        assert_eq!(iter_rev.collect::<Vec<_>>(), vec![&mut 6, &mut 5, &mut 4, &mut 3, &mut 2, &mut 1]);

        let mut iter = IterMut {
            head: head.iter_mut(),
            tail: tail.iter_mut(),
        };

        assert_eq!(iter.len(), 6);
        assert_eq!(iter.next(), Some(&mut 1));
        assert_eq!(iter.next_back(), Some(&mut 6));
        assert_eq!(iter.len(), 4);
        assert_eq!(iter.next(), Some(&mut 2));
        assert_eq!(iter.next_back(), Some(&mut 5));
        assert_eq!(iter.len(), 2);
        assert_eq!(iter.next(), Some(&mut 3));
        assert_eq!(iter.next_back(), Some(&mut 4));
        assert_eq!(iter.len(), 0);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
    }
}
