use std::marker::PhantomData;

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
pub struct Fixed<E, B>
where
    E: Copy + PartialEq,
    B: AsRef<[E]> + AsMut<[E]>,
{
    head: usize,
    buffer: B,
    _marker: PhantomData<E>,
}

impl<E, B> Fixed<E, B>
where
    E: Copy + PartialEq,
    B: AsRef<[E]> + AsMut<[E]>,
{
    /// Returns the maximum number of elements this buffer can contain.
    ///
    /// ```rust
    /// use sampara::buffer::Fixed;
    ///
    /// fn main() {
    ///     let buffer = Fixed::from([1, 2, 3, 4]);
    ///     assert_eq!(buffer.capacity(), 4);
    /// }
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
    pub fn push(&mut self, item: E) -> E {
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
    pub fn get(&self, index: usize) -> &E {
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
    pub fn get_mut(&mut self, index: usize) -> &mut E {
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
            _marker: PhantomData,
        }
    }
}

impl<E, B> From<B> for Fixed<E, B>
where
    E: Copy + PartialEq,
    B: AsRef<[E]> + AsMut<[E]>,
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
