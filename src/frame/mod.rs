use crate::sample::Sample;

/// A trait for working generically across `N`-sized blocks of [`Sample`]s,
/// representing sampling values across `N` channels at a single point in time.
/// Each of these blocks is called a "frame".
pub trait Frame<const N: usize>: Copy + Clone + PartialEq {
    /// The [`Sample`] type stored in each channel within the frame.
    type Sample: Sample;

    /// The equilibrium value for this [`Frame`] type.
    ///
    /// ```rust
    /// use sampara::{Frame, Mono, Stereo};
    ///
    /// fn main() {
    ///     assert_eq!(Mono::<f32>::EQUILIBRIUM, [0.0]);
    ///     assert_eq!(Stereo::<f32>::EQUILIBRIUM, [0.0, 0.0]);
    ///     assert_eq!(<[f32; 3]>::EQUILIBRIUM, [0.0, 0.0, 0.0]);
    ///     assert_eq!(<[u8; 2]>::EQUILIBRIUM, [128u8, 128]);
    /// }
    /// ```
    const EQUILIBRIUM: Self;

    /// Creates a new [`Frame`] where `Sample` for each channel is produced by
    /// repeatedly calling the provided function.
    ///
    /// The function should map each channel index to a [`Sample`] value.
    ///
    /// ```rust
    /// use sampara::{Frame, Stereo};
    ///
    /// fn main() {
    ///     let frame = <[i8; 3]>::from_fn(|i| (i as i8 + 1) * 32);
    ///     assert_eq!(frame, [32, 64, 96]);
    ///
    ///     let frame = Stereo::<f32>::from_fn(|i| i as f32 * 0.5);
    ///     assert_eq!(frame, [0.0, 0.5]);
    /// }
    fn from_fn<F>(func: F) -> Self
    where
        F: FnMut(usize) -> Self::Sample;

    /// Creates a new [`Frame`] from a borrowed [`Iterator`] yielding samples
    /// for each channel.
    ///
    /// Returns [`None`] if the given [`Iterator`] does not yield enough
    /// [`Sample`] values.
    ///
    /// ```rust
    /// use sampara::{Frame, Stereo};
    ///
    /// fn main() {
    ///     let mut samples = (0..=6).into_iter();
    ///
    ///     let opt_frame = Stereo::<_>::from_samples(&mut samples);
    ///     assert_eq!(opt_frame, Some([0, 1]));
    ///
    ///     let opt_frame = <[i8; 4]>::from_samples(&mut samples);
    ///     assert_eq!(opt_frame, Some([2, 3, 4, 5]));
    ///
    ///     let opt_frame = <[i8; 3]>::from_samples(&mut samples);
    ///     assert_eq!(opt_frame, None);
    /// }
    fn from_samples<I>(samples: &mut I) -> Option<Self>
    where
        I: Iterator<Item = Self::Sample>;

    /// Yields a reference to the [`Sample`] in the channel at a given index,
    /// or [`None`] if it does not exist.
    ///
    /// ```rust
    /// use sampara::Frame;
    ///
    /// fn main() {
    ///     let frame = [16_u8, 32, 48, 64];
    ///     assert_eq!(frame.channel(1), Some(&32));
    ///     assert_eq!(frame.channel(3), Some(&64));
    ///     assert_eq!(frame.channel(4), None);
    /// }
    /// ```
    fn channel(&self, idx: usize) -> Option<&Self::Sample>;

    /// Like [`Self::channel()`], but yields a mutable reference instead.
    ///
    /// ```rust
    /// use sampara::Frame;
    ///
    /// fn main() {
    ///     let mut frame = [16_u8, 32, 48, 64];
    ///     *frame.channel_mut(1).unwrap() = 0;
    ///     *frame.channel_mut(3).unwrap() = 0;
    ///     assert_eq!(frame.channel_mut(4), None);
    ///     assert_eq!(frame, [16, 0, 48, 0]);
    /// }
    /// ```
    fn channel_mut(&mut self, idx: usize) -> Option<&mut Self::Sample>;

    /// Returns an iterator that yields an immutable reference to each
    /// [`Sample`] in [`Self`] in channel order.
    ///
    /// ```rust
    /// use sampara::Frame;
    ///
    /// fn main() {
    ///     let frame = [16_u8, 32, 48, 64];
    ///     for (ch, i) in frame.channels().zip(1u8..) {
    ///         // Need `&` here, iterating over references.
    ///         assert_eq!(ch, &(16 * i));
    ///     }
    /// }
    /// ```
    fn channels(&self) -> Channels<'_, Self::Sample>;

    /// Returns an iterator that yields a mutable reference to each
    /// [`Sample`] in [`Self`] in channel order.
    ///
    /// ```rust
    /// use sampara::Frame;
    ///
    /// fn main() {
    ///     let mut frame = [16_u8, 32, 48, 64];
    ///     for (ch, i) in frame.channels_mut().zip(1u8..) {
    ///         // Need `&` here, iterating over references.
    ///         assert_eq!(ch, &(16 * i));
    ///         *ch /= 16;
    ///     }
    ///
    ///     assert_eq!(frame, [1, 2, 3, 4]);
    /// }
    /// ```
    fn channels_mut(&mut self) -> ChannelsMut<'_, Self::Sample>;

    /// Consumes [`Self`] and returns an iterator that yields each [`Sample`]
    /// in [`Self`] in channel order.
    ///
    /// ```rust
    /// use sampara::Frame;
    ///
    /// fn main() {
    ///     let frame = [16_u8, 32, 48, 64];
    ///     for (ch, i) in frame.into_channels().zip(1u8..) {
    ///         // Do not need `&` here, iterating over values.
    ///         assert_eq!(ch, 16 * i);
    ///     }
    /// }
    /// ```
    fn into_channels(self) -> IntoChannels<Self::Sample, N>;

    /// Creates a new `Frame<N>` by applying a function to each [`Sample`] in
    /// [`Self`] in channel order.
    ///
    /// ```rust
    /// use sampara::Frame;
    ///
    /// fn main() {
    ///     let mapped: [u8; 4] = [2u8, 3, 5, 7].map_channels(|x| x + 1);
    ///     assert_eq!(mapped, [3, 4, 6, 8]);
    ///     assert_eq!([0.5f32].map_channels::<f32, _>(|x| x * x), 0.25);
    /// }
    /// ```
    fn map_channels<F, M>(self, mut func: M) -> F
    where
        F: Frame<N>,
        M: FnMut(Self::Sample) -> F::Sample,
    {
        let mut out = F::EQUILIBRIUM;

        for (y, x) in out.channels_mut().zip(self.into_channels()) {
            *y = func(x);
        }

        out
    }

    /// Creates a new `Frame<N>` by applying a function to each pair of
    /// [`Sample`]s in [`Self`] and another [`Frame<N>`] in channel order.
    ///
    /// ```rust
    /// use sampara::frame::Frame;
    ///
    /// fn main() {
    ///     let frame_a = [-10i8, -20, -30, -40];
    ///     let frame_b = [-0.1f32, 0.2, -0.4, 0.8];
    ///
    ///     let o: [i8; 4] = frame_a.zip_map_channels(frame_b, |a, b| {
    ///         if b < 0.0 { -a }
    ///         else { (a as f32 * b) as i8 }
    ///     });
    ///     assert_eq!(o, [10, -4, 30, -32]);
    ///
    ///     let frame_a = [-10i8];
    ///     let frame_b = [-0.1f32];
    ///
    ///     let o: i8 = frame_a.zip_map_channels(frame_b, |a, b| {
    ///         if b < 0.0 { -a }
    ///         else { (a as f32 * b) as i8 }
    ///     });
    ///     assert_eq!(o, 10);
    /// }
    /// ```
    fn zip_map_channels<O, F, M>(self, other: O, mut func: M) -> F
    where
        O: Frame<N>,
        F: Frame<N>,
        M: FnMut(Self::Sample, O::Sample) -> F::Sample,
    {
        let mut out = F::EQUILIBRIUM;

        for (y, (xs, xo)) in out.channels_mut().zip(self.into_channels().zip(other.into_channels())) {
            *y = func(xs, xo);
        }

        out
    }
}

impl<S, const N: usize> Frame<N> for [S; N]
where
    S: Sample,
{
    type Sample = S;

    const EQUILIBRIUM: Self = [S::EQUILIBRIUM; N];

    fn from_fn<F>(mut func: F) -> Self
    where
        F: FnMut(usize) -> Self::Sample
    {
        let mut out = Self::EQUILIBRIUM;

        for (i, ch) in out.iter_mut().enumerate() {
            *ch = func(i);
        }

        out
    }

    fn from_samples<I>(samples: &mut I) -> Option<Self>
    where
        I: Iterator<Item = Self::Sample>
    {
        let mut out = Self::EQUILIBRIUM;

        for ch in out.iter_mut() {
            *ch = samples.next()?;
        }

        Some(out)
    }

    fn channel(&self, idx: usize) -> Option<&Self::Sample> {
        self.get(idx)
    }

    fn channel_mut(&mut self, idx: usize) -> Option<&mut Self::Sample> {
        self.get_mut(idx)
    }

    #[inline]
    fn channels(&self) -> Channels<'_, Self::Sample> {
        Channels(self.iter())
    }

    #[inline]
    fn channels_mut(&mut self) -> ChannelsMut<'_, Self::Sample> {
        ChannelsMut(self.iter_mut())
    }

    #[inline]
    fn into_channels(self) -> IntoChannels<Self::Sample, N> {
        // TODO: Temporary use of `new` method while this is still unstable.
        //       Replace with more ergonomic method once it lands in rustc.
        IntoChannels(std::array::IntoIter::new(self))
    }
}

impl<S> Frame<1> for S
where
    S: Sample,
{
    type Sample = S;

    const EQUILIBRIUM: Self = S::EQUILIBRIUM;

    fn from_fn<F>(mut func: F) -> Self
    where
        F: FnMut(usize) -> Self::Sample
    {
        func(0)
    }

    fn from_samples<I>(samples: &mut I) -> Option<Self>
    where
        I: Iterator<Item = Self::Sample>
    {
        samples.next()
    }

    fn channel(&self, idx: usize) -> Option<&Self::Sample> {
        if idx == 0 { Some(self) }
        else { None }
    }

    fn channel_mut(&mut self, idx: usize) -> Option<&mut Self::Sample> {
        if idx == 0 { Some(self) }
        else { None }
    }

    #[inline]
    fn channels(&self) -> Channels<'_, Self::Sample> {
        Channels(core::slice::from_ref(self).iter())
    }

    #[inline]
    fn channels_mut(&mut self) -> ChannelsMut<'_, Self::Sample> {
        ChannelsMut(core::slice::from_mut(self).iter_mut())
    }

    #[inline]
    fn into_channels(self) -> IntoChannels<Self::Sample, 1> {
        // TODO: Temporary use of `new` method while this is still unstable.
        //       Replace with more ergonomic method once it lands in rustc.
        IntoChannels(std::array::IntoIter::new([self]))
    }
}

pub type Mono<S> = [S; 1];
pub type Stereo<S> = [S; 2];

/// An iterator that yields the [`Sample`] for each channel in the frame by
/// reference.
#[derive(Clone)]
pub struct Channels<'a, S: Sample>(core::slice::Iter<'a, S>);

impl<'a, S: Sample> Iterator for Channels<'a, S> {
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

impl<'a, S: Sample> ExactSizeIterator for Channels<'a, S> {
    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a, S: Sample> DoubleEndedIterator for Channels<'a, S> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

/// Like [`Channels`], but yields mutable references instead.
pub struct ChannelsMut<'a, S: Sample>(core::slice::IterMut<'a, S>);

impl<'a, S: Sample> Iterator for ChannelsMut<'a, S> {
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

impl<'a, S: Sample> ExactSizeIterator for ChannelsMut<'a, S> {
    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a, S: Sample> DoubleEndedIterator for ChannelsMut<'a, S> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

/// Like [`Channels`], but yields owned [`Sample`]s instead of references.
#[derive(Clone)]
pub struct IntoChannels<S: Sample, const N: usize>(std::array::IntoIter<S, N>);

impl<S: Sample, const N: usize> Iterator for IntoChannels<S, N> {
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

impl<S: Sample, const N: usize> ExactSizeIterator for IntoChannels<S, N> {
    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<S: Sample, const N: usize> DoubleEndedIterator for IntoChannels<S, N> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}
