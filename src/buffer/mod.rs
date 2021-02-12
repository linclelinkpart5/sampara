pub trait Buffer {
    /// The type contained within the buffer.
    type Element;

    /// Borrows the buffer as a slice.
    fn as_slice(&self) -> &[Self::Element];
}

pub trait BufferMut: Buffer {
    /// Borrows the buffer as a mutable slice.
    fn as_slice_mut(&mut self) -> &mut [Self::Element];
}

impl<'a, T> Buffer for &'a [T] {
    type Element = T;

    #[inline]
    fn as_slice(&self) -> &[Self::Element] {
        self
    }
}

impl<'a, T> Buffer for &'a mut [T] {
    type Element = T;

    #[inline]
    fn as_slice(&self) -> &[Self::Element] {
        self
    }
}

impl<'a, T> BufferMut for &'a mut [T] {
    #[inline]
    fn as_slice_mut(&mut self) -> &mut [Self::Element] {
        self
    }
}

impl<T, const N: usize> Buffer for [T; N] {
    type Element = T;

    #[inline]
    fn as_slice(&self) -> &[Self::Element] {
        &self[..]
    }
}

impl<T, const N: usize> BufferMut for [T; N] {
    #[inline]
    fn as_slice_mut(&mut self) -> &mut [Self::Element] {
        &mut self[..]
    }
}

impl<'a, T, const N: usize> Buffer for &'a [T; N] {
    type Element = T;

    #[inline]
    fn as_slice(&self) -> &[Self::Element] {
        &self[..]
    }
}

impl<'a, T, const N: usize> Buffer for &'a mut [T; N] {
    type Element = T;

    #[inline]
    fn as_slice(&self) -> &[Self::Element] {
        &self[..]
    }
}

impl<'a, T, const N: usize> BufferMut for &'a mut [T; N] {
    #[inline]
    fn as_slice_mut(&mut self) -> &mut [Self::Element] {
        &mut self[..]
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Const<E, const N: usize>
where
    E: Copy + PartialEq,
{
    curr_idx: usize,
    data: [E; N],
}

impl<E, const N: usize> Const<E, N>
where
    E: Copy + PartialEq,
{
    pub fn new(initial: [E; N]) -> Self {
        Self { data: initial, curr_idx: 0 }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        N
    }

    /// Pushes a new element onto the rear of the buffer, and pops off and
    /// returns the replaced element from the front.
    ///
    /// If the buffer has a constant length of 0, this always returns the
    /// element that was just attempted to be pushed.
    ///
    /// ```rust
    /// use sampara::buffer::Const;
    ///
    /// fn main() {
    ///     let mut buffer = Const::new([1, 2, 3]);
    ///     assert_eq!(buffer.push(4), 1);
    ///     assert_eq!(buffer.push(5), 2);
    ///     assert_eq!(buffer.push(6), 3);
    ///     assert_eq!(buffer.push(7), 4);
    ///     assert_eq!(buffer.push(8), 5);
    ///     assert_eq!(buffer.push(9), 6);
    ///
    ///     // An empty `Const` buffer always returns the element that was just
    ///     // attempted to be pushed.
    ///     let mut empty = Const::new([0; 0]);
    ///     assert_eq!(empty.push(27), 27);
    ///     assert_eq!(empty.push(42), 42);
    ///     assert_eq!(empty.push(69), 69);
    /// }
    /// ```
    pub fn push(&mut self, item: E) -> E {
        if N == 0 {
            // Buffer has zero length, just re-return the passed-in element.
            return item;
        }

        let mut next_idx = self.curr_idx + 1;
        if next_idx >= N {
            next_idx = 0;
        }

        // Bounds checking can be skipped safely since the length is constant.
        let old_item = unsafe {
            std::mem::replace(self.data.get_unchecked_mut(self.curr_idx), item)
        };
        self.curr_idx = next_idx;
        old_item
    }

    /// Returns a view of two front and rear slices that make up the buffer as
    /// slices.
    ///
    /// These two slices chained together represent all elements within the
    /// buffer in order.
    ///
    /// The first slice is always aligned contiguously behind the second slice.
    ///
    /// ```rust
    /// use sampara::buffer::Const;
    ///
    /// fn main() {
    ///     let mut buffer = Const::new([0; 4]);
    ///     assert_eq!(buffer.slices(), (&[0, 0, 0, 0][..], &[][..]));
    ///     buffer.push(1);
    ///     buffer.push(2);
    ///     assert_eq!(buffer.slices(), (&[0, 0][..], &[1, 2][..]));
    ///     buffer.push(3);
    ///     buffer.push(4);
    ///     assert_eq!(buffer.slices(), (&[1, 2, 3, 4][..], &[][..]));
    /// }
    /// ```
    #[inline]
    pub fn slices(&self) -> (&[E], &[E]) {
        let (end, start) = self.data.split_at(self.curr_idx);
        (start, end)
    }

    /// Same as `.slices`, but returns mutable slices instead.
    ///
    /// ```rust
    /// use sampara::buffer::Const;
    ///
    /// fn main() {
    ///     let mut buffer = Const::new([0; 4]);
    ///
    ///     let (mut front, mut rear) = buffer.slices_mut();
    ///     *front.get_mut(2).unwrap() = 9;
    ///     assert_eq!((front, rear), (&mut [0, 0, 9, 0][..], &mut [][..]));
    ///
    ///     buffer.push(1);
    ///     buffer.push(2);
    ///     let (mut front, mut rear) = buffer.slices_mut();
    ///     *rear.get_mut(0).unwrap() = 8;
    ///     assert_eq!((front, rear), (&mut [9, 0][..], &mut [8, 2][..]));
    /// }
    /// ```
    #[inline]
    pub fn slices_mut(&mut self) -> (&mut [E], &mut [E]) {
        let (end, start) = self.data.split_at_mut(self.curr_idx);
        (start, end)
    }
}
