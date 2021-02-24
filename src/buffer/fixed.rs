use std::iter::FusedIterator;
use std::slice::{Iter as SliceIter, IterMut as SliceIterMut};

use crate::buffer::Buffer;

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
#[derive(Copy, Clone, Debug)]
pub struct Fixed<B>
where
    B: Buffer,
{
    head: usize,
    buffer: B,
}

impl<B> Fixed<B>
where
    B: Buffer,
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
    pub fn fill(&mut self, item: B::Item) {
        self.buffer.as_mut().fill(item);
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
    pub fn fill_with<F>(&mut self, func: F)
    where
        F: FnMut() -> B::Item,
    {
        self.buffer.as_mut().fill_with(func);
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
        self.buffer.as_ref().len()
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
    pub fn push(&mut self, item: B::Item) -> B::Item {
        if self.capacity() == 0 {
            // Buffer has zero capacity, just re-return the passed-in element.
            return item;
        }

        let mut next_head = self.head + 1;
        if next_head >= self.capacity() {
            next_head = 0;
        }

        // Bounds checking can be skipped safely since the length is constant.
        let old_item = unsafe {
            std::mem::replace(self.buffer.as_mut().get_unchecked_mut(self.head), item)
        };
        self.head = next_head;
        old_item
    }

    fn wrapped(&self, index: usize) -> usize {
        (self.head + index).checked_rem(self.capacity()).unwrap()
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
    pub fn get(&self, index: usize) -> &B::Item {
        let wrapped_index = self.wrapped(index);
        &self.buffer.as_ref()[wrapped_index]
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
    pub fn get_mut(&mut self, index: usize) -> &mut B::Item {
        let wrapped_index = self.wrapped(index);
        &mut self.buffer.as_mut()[wrapped_index]
    }

    /// Constructs a [`Fixed`] ring buffer from a given inner buffer and
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
    pub fn from_raw_parts(head: usize, buffer: B) -> Self {
        let wrapped_head = head.checked_rem(buffer.as_ref().len()).unwrap_or(0);

        Self {
            head: wrapped_head,
            buffer,
        }
    }

    /// Returns the head index and a reference to the inner buffer.
    ///
    /// ```rust
    /// use sampara::buffer::Fixed;
    ///
    /// fn main() {
    ///     let mut buffer = Fixed::from([0, 1, 2]);
    ///     buffer.push(6);
    ///     buffer.push(7);
    ///
    ///     assert_eq!(buffer.raw_parts(), (2, &[6, 7, 2]));
    /// }
    /// ```
    #[inline]
    pub fn raw_parts(&self) -> (usize, &B) {
        let Self { head, buffer } = self;
        (*head, buffer)
    }

    /// Returns a reference to the inner buffer.
    ///
    /// ```rust
    /// use sampara::buffer::Fixed;
    ///
    /// fn main() {
    ///     let mut buffer = Fixed::from([0, 1, 2]);
    ///     buffer.push(6);
    ///     buffer.push(7);
    ///
    ///     assert_eq!(buffer.buffer(), &[6, 7, 2]);
    /// }
    /// ```
    #[inline]
    pub fn buffer(&self) -> &B {
        let (_, buffer) = self.raw_parts();
        buffer
    }

    /// Decomposes a [`Fixed`] ring buffer into a head index and inner buffer.
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
    pub fn into_raw_parts(self) -> (usize, B) {
        let Self { head, buffer } = self;
        (head, buffer)
    }

    /// Decomposes a [`Fixed`] ring buffer into an inner buffer.
    ///
    /// ```rust
    /// use sampara::buffer::Fixed;
    ///
    /// fn main() {
    ///     let mut buffer = Fixed::from([0, 1, 2]);
    ///     buffer.push(6);
    ///     buffer.push(7);
    ///
    ///     assert_eq!(buffer.into_buffer(), [6, 7, 2]);
    /// }
    /// ```
    #[inline]
    pub fn into_buffer(self) -> B {
        let (_, buffer) = self.into_raw_parts();
        buffer
    }

    fn as_slices(&self) -> (&[B::Item], &[B::Item]) {
        let (tail, head) = self.buffer.as_ref().split_at(self.head);
        (head, tail)
    }

    fn as_slices_mut(&mut self) -> (&mut [B::Item], &mut [B::Item]) {
        let (tail, head) = self.buffer.as_mut().split_at_mut(self.head);
        (head, tail)
    }

    /// Returns an iterator that yields references to the items in this buffer,
    /// in order.
    ///
    /// ```rust
    /// use sampara::buffer::Fixed;
    ///
    /// fn main() {
    ///     let mut buffer = Fixed::from([1, 2, 3, 4]);
    ///
    ///     let iter = buffer.iter();
    ///     assert_eq!(iter.collect::<Vec<_>>(), vec![&1, &2, &3, &4]);
    ///
    ///     buffer.push(5);
    ///     buffer.push(6);
    ///     let iter = buffer.iter();
    ///     assert_eq!(iter.collect::<Vec<_>>(), vec![&3, &4, &5, &6]);
    /// }
    /// ```
    pub fn iter(&self) -> Iter<'_, B::Item> {
        let (head, tail) = self.as_slices();

        Iter {
            head: head.iter(),
            tail: tail.iter(),
        }
    }

    /// Similar to [`iter`], but with mutable references instead.
    ///
    /// ```rust
    /// use sampara::buffer::Fixed;
    ///
    /// fn main() {
    ///     let mut buffer = Fixed::from([1, 2, 3, 4]);
    ///
    ///     for x in buffer.iter_mut() {
    ///         *x *= 11;
    ///     }
    ///
    ///     let iter = buffer.iter();
    ///     assert_eq!(buffer.iter().collect::<Vec<_>>(), &[&11, &22, &33, &44]);
    ///
    ///     buffer.push(5);
    ///     buffer.push(6);
    ///     for x in buffer.iter_mut() {
    ///         *x += 100;
    ///     }
    ///
    ///     let iter = buffer.iter_mut();
    ///     assert_eq!(iter.collect::<Vec<_>>(), vec![&133, &144, &105, &106]);
    /// }
    /// ```
    pub fn iter_mut(&mut self) -> IterMut<'_, B::Item> {
        let (head, tail) = self.as_slices_mut();

        IterMut {
            head: head.iter_mut(),
            tail: tail.iter_mut(),
        }
    }
}

impl<B> From<B> for Fixed<B>
where
    B: Buffer,
{
    /// Constructs a [`Fixed`] ring buffer from a given inner buffer.
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
    fn from(buffer: B) -> Self {
        Self::from_raw_parts(0, buffer)
    }
}

impl<B> AsRef<[B::Item]> for Fixed<B>
where
    B: Buffer,
{
    fn as_ref(&self) -> &[B::Item] {
        self.buffer.as_ref()
    }
}

impl<B> AsMut<[B::Item]> for Fixed<B>
where
    B: Buffer,
{
    fn as_mut(&mut self) -> &mut [B::Item] {
        self.buffer.as_mut()
    }
}

// impl<BA, BB> PartialEq<Fixed<BB>> for Fixed<BA>
// where
//     BA: Buffer,
//     BB: Buffer,
//     BA::Item: PartialEq<BB::Item>,
// {
//     fn eq(&self, other: &Fixed<BB>) -> bool {
//         self.iter().eq(other.iter())
//     }
// }

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
