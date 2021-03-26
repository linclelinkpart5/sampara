use crate::sample::Sample;

/// A trait for working generically across `N`-sized blocks of [`Sample`]s,
/// representing sampling values across `N` channels at a single point in time.
/// Each of these blocks is called a "frame".
pub trait Frame<const N: usize>: Copy + Clone + PartialEq {
    /// The [`Sample`] type stored in each channel within the frame.
    type Sample: Sample;

    /// A [`Frame`] type that has the same number of channels as [`Self`], but
    /// with the associated [`Sample::Signed`] sample format.
    type Signed: Frame<N, Sample = <Self::Sample as Sample>::Signed>;

    /// A [`Frame`] type that has the same number of channels as [`Self`], but
    /// with the associated [`Sample::Float`] sample format.
    type Float: Frame<N, Sample = <Self::Sample as Sample>::Float>;

    /// The equilibrium value for this [`Frame`] type.
    ///
    /// ```
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
    /// ```
    /// use sampara::{Frame, Stereo};
    ///
    /// fn main() {
    ///     let frame = <[i8; 3]>::from_fn(|i| (i as i8 + 1) * 32);
    ///     assert_eq!(frame, [32, 64, 96]);
    ///
    ///     let frame = Stereo::<f32>::from_fn(|i| i as f32 * 0.5);
    ///     assert_eq!(frame, [0.0, 0.5]);
    /// }
    /// ```
    fn from_fn<F>(func: F) -> Self
    where
        F: FnMut(usize) -> Self::Sample;

    /// Creates a new [`Frame`] from a borrowed [`Iterator`] yielding samples
    /// for each channel.
    ///
    /// Returns [`None`] if the given [`Iterator`] does not yield enough
    /// [`Sample`] values.
    ///
    /// ```
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
    /// ```
    fn from_samples<I>(samples: &mut I) -> Option<Self>
    where
        I: Iterator<Item = Self::Sample>;

    /// Yields a reference to the [`Sample`] in the channel at a given index,
    /// or [`None`] if it does not exist.
    ///
    /// ```
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
    /// ```
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
    /// ```
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
    /// ```
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
    /// ```
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
    /// This would ideally be called `map`, but that name conflicts with an
    /// unstable method on arrays in the Rust stdlib.
    ///
    /// ```
    /// use sampara::Frame;
    ///
    /// fn main() {
    ///     let mapped: [u8; 4] = [2u8, 3, 5, 7].apply(|x| x + 1);
    ///     assert_eq!(mapped, [3, 4, 6, 8]);
    ///
    ///     let mapped: f32 = [0.5f32].apply(|x| x * x);
    ///     assert_eq!(mapped, 0.25);
    /// }
    /// ```
    fn apply<F, M>(self, mut func: M) -> F
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

    /// Mutates [`Self`] in-place by applying a function to each [`Sample`] in
    /// [`Self`] in channel order.
    ///
    /// ```
    /// use sampara::Frame;
    ///
    /// fn main() {
    ///     let mut frame = [2u8, 3, 5, 7];
    ///     frame.transform(|x| x + 1);
    ///     assert_eq!(frame, [3, 4, 6, 8]);
    ///
    ///     let mut frame = 0.5f32;
    ///     frame.transform(|x| x * x);
    ///     assert_eq!(frame, 0.25);
    /// }
    /// ```
    fn transform<M>(&mut self, mut func: M)
    where
        M: FnMut(Self::Sample) -> Self::Sample,
    {
        for x in self.channels_mut() {
            *x = func(*x);
        }
    }

    /// Creates a new `Frame<N>` by applying a function to each pair of
    /// [`Sample`]s in [`Self`] and another [`Frame<N>`] in channel order.
    ///
    /// ```
    /// use sampara::frame::Frame;
    ///
    /// fn main() {
    ///     let frame_a = [-10i8, -20, -30, -40];
    ///     let frame_b = [-0.1f32, 0.2, -0.4, 0.8];
    ///
    ///     let o: [i8; 4] = frame_a.zip_apply(frame_b, |a, b| {
    ///         if b < 0.0 { -a }
    ///         else { (a as f32 * b) as i8 }
    ///     });
    ///     assert_eq!(o, [10, -4, 30, -32]);
    ///
    ///     let frame_a = [-10i8];
    ///     let frame_b = [-0.1f32];
    ///
    ///     let o: i8 = frame_a.zip_apply(frame_b, |a, b| {
    ///         if b < 0.0 { -a }
    ///         else { (a as f32 * b) as i8 }
    ///     });
    ///     assert_eq!(o, 10);
    /// }
    /// ```
    fn zip_apply<O, F, M>(self, other: O, mut func: M) -> F
    where
        O: Frame<N>,
        F: Frame<N>,
        M: FnMut(Self::Sample, O::Sample) -> F::Sample,
    {
        let mut out = F::EQUILIBRIUM;

        let pairs = self.into_channels().zip(other.into_channels());
        for (y, (xs, xo)) in out.channels_mut().zip(pairs) {
            *y = func(xs, xo);
        }

        out
    }

    /// Mutates [`Self`] in-place by applying a function to each pair of
    /// [`Sample`]s in [`Self`] and another [`Frame<N>`] in channel order.
    ///
    /// ```
    /// use sampara::Frame;
    ///
    /// fn main() {
    ///     let mut frame_a = [2u8, 3, 5, 7];
    ///     let frame_b = [3u8, 2, 1, 0];
    ///     frame_a.zip_transform(frame_b, |a, b| a * b + 1);
    ///     assert_eq!(frame_a, [7, 7, 6, 1]);
    ///
    ///     let mut frame_a = 0.3f32;
    ///     let frame_b = [0.4];
    ///     frame_a.zip_transform(frame_b, |a, b| a * b + 0.5);
    ///     assert_eq!(frame_a, 0.62);
    /// }
    /// ```
    fn zip_transform<O, M>(&mut self, other: O, mut func: M)
    where
        O: Frame<N>,
        M: FnMut(Self::Sample, O::Sample) -> Self::Sample,
    {
        for (x, o) in self.channels_mut().zip(other.into_channels()) {
            *x = func(*x, o);
        }
    }

    /// Converts [`Self`] into its equivalent [`Self::Signed`] format.
    ///
    /// ```
    /// use sampara::Frame;
    ///
    /// fn main() {
    ///     assert_eq!([128u8; 2].into_signed_frame(), [0i8; 2]);
    ///     assert_eq!([-64i8, 64].into_signed_frame(), [-64i8, 64]);
    /// }
    /// ```
    fn into_signed_frame(self) -> Self::Signed {
        self.apply(Sample::into_signed_sample)
    }

    /// Converts [`Self`] into its equivalent [`Self::Float`] format.
    ///
    /// ```
    /// use sampara::Frame;
    ///
    /// fn main() {
    ///     assert_eq!([128u8; 2].into_float_frame(), [0.0, 0.0]);
    ///     assert_eq!([-64i8, 64].into_float_frame(), [-0.5, 0.5]);
    /// }
    /// ```
    fn into_float_frame(self) -> Self::Float {
        self.apply(Sample::into_float_sample)
    }

    /// Adds/offsets the amplitude of each channel in [`Self`] by a signed
    /// amplitude.
    ///
    /// ```
    /// use sampara::Frame;
    ///
    /// fn main() {
    ///     assert_eq!([0.25, -0.5].add_amp(0.5), [0.75, 0.0]);
    ///     assert_eq!([0.5, -0.25].add_amp(-0.25), [0.25, -0.5]);
    ///     assert_eq!([128u8, 192].add_amp(-64), [64, 128]);
    /// }
    /// ```
    #[inline]
    fn add_amp(self, amp: <Self::Sample as Sample>::Signed) -> Self {
        self.apply(|s| Sample::add_amp(s, amp))
    }

    /// Multiplies/scales the amplitude of each channel in [`Self`] by a float
    /// amplitude.
    ///
    /// ```
    /// use sampara::Frame;
    ///
    /// fn main() {
    ///     assert_eq!([0.25, -0.5].mul_amp(0.5), [0.125, -0.25]);
    ///     assert_eq!([0.5, -0.25].mul_amp(-0.25), [-0.125, 0.0625]);
    ///     assert_eq!([128u8, 192].mul_amp(0.4), [128, 153]);
    /// }
    /// ```
    #[inline]
    fn mul_amp(self, amp: <Self::Sample as Sample>::Float) -> Self {
        self.apply(|s| Sample::mul_amp(s, amp))
    }

    /// Adds/offsets the amplitude of each channel in [`Self`] with each
    /// corresponding channel in a given [`Self::Signed`].
    ///
    /// ```
    /// use sampara::Frame;
    ///
    /// fn main() {
    ///     assert_eq!([0.25, -0.5].add_frame([0.5, 0.75]), [0.75, 0.25]);
    ///     assert_eq!([0.5, -0.25].add_frame([-0.25, 0.5]), [0.25, 0.25]);
    ///     assert_eq!([128u8, 192].add_frame([-64i8, -64]), [64, 128]);
    /// }
    /// ```
    #[inline]
    fn add_frame(self, amps: Self::Signed) -> Self {
        self.zip_apply(amps, |a, b| Sample::add_amp(a, b))
    }

    /// Multiplies/scales the amplitude of each channel in [`Self`] with each
    /// corresponding channel in a given [`Self::Float`].
    ///
    /// ```
    /// use sampara::Frame;
    ///
    /// fn main() {
    ///     assert_eq!([0.25, -0.5].mul_frame([0.5, 0.75]), [0.125, -0.375]);
    ///     assert_eq!([0.5, -0.25].mul_frame([-0.25, 0.5]), [-0.125, -0.125]);
    ///     assert_eq!([128u8, 192].mul_frame([0.4, 0.2]), [128, 140]);
    /// }
    /// ```
    #[inline]
    fn mul_frame(self, amps: Self::Float) -> Self {
        self.zip_apply(amps, |a, b| Sample::mul_amp(a, b))
    }
}

impl<S, const N: usize> Frame<N> for [S; N]
where
    S: Sample,
{
    type Sample = S;

    type Signed = [S::Signed; N];
    type Float = [S::Float; N];

    const EQUILIBRIUM: Self = [S::EQUILIBRIUM; N];

    fn from_fn<F>(mut func: F) -> Self
    where
        F: FnMut(usize) -> Self::Sample,
    {
        let mut out = Self::EQUILIBRIUM;

        for (i, ch) in out.iter_mut().enumerate() {
            *ch = func(i);
        }

        out
    }

    fn from_samples<I>(samples: &mut I) -> Option<Self>
    where
        I: Iterator<Item = Self::Sample>,
    {
        let mut out = Self::EQUILIBRIUM;

        for ch in out.iter_mut() {
            *ch = samples.next()?;
        }

        Some(out)
    }

    #[inline]
    fn channel(&self, idx: usize) -> Option<&Self::Sample> {
        self.get(idx)
    }

    #[inline]
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

    type Signed = S::Signed;
    type Float = S::Float;

    const EQUILIBRIUM: Self = S::EQUILIBRIUM;

    fn from_fn<F>(mut func: F) -> Self
    where
        F: FnMut(usize) -> Self::Sample,
    {
        func(0)
    }

    fn from_samples<I>(samples: &mut I) -> Option<Self>
    where
        I: Iterator<Item = Self::Sample>,
    {
        samples.next()
    }

    fn channel(&self, idx: usize) -> Option<&Self::Sample> {
        if idx == 0 {
            Some(self)
        } else {
            None
        }
    }

    fn channel_mut(&mut self, idx: usize) -> Option<&mut Self::Sample> {
        if idx == 0 {
            Some(self)
        } else {
            None
        }
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

// TODO: Implement this once associated consts can be used as const generics
//       (i.e. when `N` does not need to be specified as a const generic param)!
// impl<A, B, const N: usize> From<A> for B
// where
//     A: Frame<N>,
//     B: Frame<N>,
//     B::Sample: ConvertFrom<A::Sample>,
// {
//     fn from(value: A) -> B {
//         value.apply(ConvertInto::convert_into)
//     }
// }

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
