mod adaptors;
mod generators;
mod iterators;

use crate::biquad::Params;
use crate::buffer::Buffer;
use crate::components::{Calculator, Combinator, Processor};
use crate::interpolate::Interpolator;
use crate::sample::FloatSample;
use crate::{Duplex, Frame, Sample};

// use crate::combinators as combs;
use crate::processors as procs;

pub use adaptors::*;
pub use generators::*;
pub use iterators::*;

// pub type Map<S, FO, M, const NI: usize, const NO: usize> =
//     Process<
//         S,
//         procs::Map<
//             <S as Signal<NI>>::Frame,
//             FO, M, NI, NO,
//         >,
//         NI, NO,
//     >;

// pub type Mix<SL, SR, FO, M, const NL: usize, const NR: usize, const NO: usize> =
//     Combine<
//         SL, SR,
//         combs::Mix<
//             <SL as Signal<NL>>::Frame,
//             <SR as Signal<NR>>::Frame,
//             FO, M, NL, NR, NO,
//         >,
//         NL, NR, NO,
//     >;

// pub type Select<SL, SR, const N: usize> =
//     Combine<SL, SR, combs::Selector<<SL as Signal<N>>::Frame, N>, N, N, N>;

/// Types that yield a sequence of [`Frame`]s, representing an audio signal.
///
/// This trait is inspired by the [`Iterator`] trait and has similar methods
/// and adaptors, but with a DSP-related focus.
pub trait Signal<const N: usize> {
    /// The [`Frame`] type returned by this [`Signal`].
    type Frame: Frame<N>;

    /// Advances [`Self`] and returns the next [`Frame`], or [`None`] if there
    /// are no more to yield.
    fn next(&mut self) -> Option<Self::Frame>;

    /// Similar to [`Self::next`], but will always yield a [`Frame`]. Yields
    /// [`Frame::EQUILIBRIUM`] if there are no more actual [`Frame`]s to yield.
    fn sig_next(&mut self) -> Self::Frame {
        self.next().unwrap_or(Frame::EQUILIBRIUM)
    }

    /// Returns the `n`th [`Frame`] from this [`Signal`], starting at 0. This
    /// will advance the [`Signal`], so multiple calls to `nth` with the same
    /// `n` will give different results.
    ///
    /// ```
    /// use sampara::{signal, Signal};
    ///
    /// fn main() {
    ///     let mut signal = signal::from_frames(0..=9);
    ///
    ///     assert_eq!(signal.nth(3), Some(3));
    ///     assert_eq!(signal.nth(3), Some(7));
    ///     assert_eq!(signal.nth(3), None);
    /// }
    /// ```
    fn nth(&mut self, n: usize) -> Option<Self::Frame> {
        self.advance_by(n).ok()?;
        self.next()
    }

    /// Borrows this [`Signal`] rather than consuming it.
    ///
    /// This is useful for applying adaptors while still retaining ownership of
    /// the original [`Signal`].
    ///
    /// ```
    /// use sampara::{signal, Signal};
    ///
    /// fn main() {
    ///     let mut signal = signal::from_frames(vec![0, 1, 2, 3]);
    ///     assert_eq!(signal.next(), Some(0));
    ///     assert_eq!(signal.by_ref().add_amp(10).next(), Some(11));
    ///     assert_eq!(signal.by_ref().mul_amp(2.5_f32).next(), Some(5));
    ///     assert_eq!(signal.next(), Some(3));
    /// }
    /// ```
    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }

    /// Creates a new [`Signal`] that applies a function to each [`Frame`] of
    /// [`Self`].
    ///
    /// ```
    /// use sampara::{signal, Signal};
    ///
    /// fn main() {
    ///     let signal = signal::from_frames(0i32..=3);
    ///     let mut mapped = signal.map(|f| f * f);
    ///
    ///     assert_eq!(mapped.next(), Some(0));
    ///     assert_eq!(mapped.next(), Some(1));
    ///     assert_eq!(mapped.next(), Some(4));
    ///     assert_eq!(mapped.next(), Some(9));
    ///     assert_eq!(mapped.next(), None);
    /// }
    /// ```
    fn map<M, FO, const NO: usize>(self, func: M) -> Map<Self, M, FO, N, NO>
    where
        Self: Sized,
        FO: Frame<NO>,
        M: FnMut(Self::Frame) -> FO,
    {
        let processed = self.process(procs::Map::new(func));
        let wrapped = Map(processed);
        wrapped
    }

    // /// Creates a new [`Signal`] that applies a function to each pair of
    // /// [`Frame`]s from [`Self`] and another input [`Signal`], in lockstep.
    // ///
    // /// If either input [`Signal`] becomes exhausted, this will also become
    // /// exhausted.
    // ///
    // /// ```
    // /// use sampara::{signal, Signal};
    // ///
    // /// fn main() {
    // ///     let signal_l = signal::from_frames(0i32..=3);
    // ///     let signal_r = signal::from_frames(4i32..);
    // ///     let mut mixed = signal_l.mix(signal_r, |l, r| [l + r, l * r]);
    // ///
    // ///     assert_eq!(mixed.next(), Some([4, 0]));
    // ///     assert_eq!(mixed.next(), Some([6, 5]));
    // ///     assert_eq!(mixed.next(), Some([8, 12]));
    // ///     assert_eq!(mixed.next(), Some([10, 21]));
    // ///
    // ///     // At this point `signal_l` is exhausted, so this is as well.
    // ///     assert_eq!(mixed.next(), None);
    // /// }
    // /// ```
    // fn mix<S, FO, M, const NR: usize, const NO: usize>(self, other: S, func: M)
    //     -> Mix<Self, S, FO, M, N, NR, NO>
    // where
    //     Self: Sized,
    //     S: Signal<NR>,
    //     FO: Frame<NO>,
    //     M: FnMut(Self::Frame, S::Frame) -> FO,
    // {
    //     let combinator = combs::Mix::new(func);
    //     self.combine(other, combinator)
    // }

    // /// Creates a new [`Signal`] that takes pairs of [`Frame`]s from [`Self`]
    // /// and another input [`Signal`] in lockstep, and returns one of them
    // /// (left or right) based on a selector switch.
    // ///
    // /// The resulting [`Signal`] will start off yielding only [`Frame`]s from
    // /// [`Self`] (left), but this can be changed at runtime.
    // ///
    // /// ```
    // /// use sampara::{signal, Signal};
    // ///
    // /// fn main() {
    // ///     let signal_l = signal::from_frames(10i32..);
    // ///     let signal_r = signal::from_frames(20i32..);
    // ///
    // ///     let mut selected = signal_l.select_left(signal_r);
    // ///     assert_eq!(selected.next(), Some(10));
    // ///     assert_eq!(selected.next(), Some(11));
    // ///     assert_eq!(selected.next(), Some(12));
    // ///
    // ///     selected.state_mut().toggle();
    // ///     assert_eq!(selected.next(), Some(23));
    // ///     assert_eq!(selected.next(), Some(24));
    // ///     assert_eq!(selected.next(), Some(25));
    // /// }
    // /// ```
    // fn select_left<S>(self, other: S) -> Select<Self, S, N>
    // where
    //     Self: Sized,
    //     S: Signal<N, Frame = Self::Frame>,
    // {
    //     let combinator = combs::Selector::left();
    //     self.combine(other, combinator)
    // }

    // /// Similar to [`Self::select_left`], except starts off yielding the right
    // /// [`Frame`]s.
    // ///
    // /// ```
    // /// use sampara::{signal, Signal};
    // ///
    // /// fn main() {
    // ///     let signal_l = signal::from_frames(10i32..);
    // ///     let signal_r = signal::from_frames(20i32..);
    // ///
    // ///     let mut selected = signal_l.select_right(signal_r);
    // ///     assert_eq!(selected.next(), Some(20));
    // ///     assert_eq!(selected.next(), Some(21));
    // ///     assert_eq!(selected.next(), Some(22));
    // ///
    // ///     selected.state_mut().toggle();
    // ///     assert_eq!(selected.next(), Some(13));
    // ///     assert_eq!(selected.next(), Some(14));
    // ///     assert_eq!(selected.next(), Some(15));
    // /// }
    // /// ```
    // fn select_right<S>(self, other: S) -> Select<Self, S, N>
    // where
    //     Self: Sized,
    //     S: Signal<N, Frame = Self::Frame>,
    // {
    //     let combinator = combs::Selector::right();
    //     self.combine(other, combinator)
    // }

    /// Creates a new [`Signal`] that yields the sum of pairs of [`Frame`]s
    /// yielded by [`Self`] and another [`Signal`] in lockstep.
    fn add_signal<B>(self, other: B) -> AddSignal<Self, B, N>
    where
        Self: Sized,
        B: Signal<N>,
        Self::Frame: Frame<N, Signed = <B::Frame as Frame<N>>::Signed>,
    {
        AddSignal {
            signal_a: self,
            signal_b: other,
        }
    }

    /// Creates a new [`Signal`] that yields the product of pairs of [`Frame`]s
    /// yielded by [`Self`] and another [`Signal`] in lockstep.
    fn mul_signal<B>(self, other: B) -> MulSignal<Self, B, N>
    where
        Self: Sized,
        B: Signal<N>,
        Self::Frame: Frame<N, Float = <B::Frame as Frame<N>>::Float>,
    {
        MulSignal {
            signal_a: self,
            signal_b: other,
        }
    }

    /// Creates a new [`Signal`] that yields each [`Frame`] of a [`Signal`]
    /// summed with a constant [`Frame`].
    fn add_frame<F>(self, frame: F) -> AddFrame<Self, F, N>
    where
        Self: Sized,
        Self::Frame: Frame<N, Signed = F>,
        F: Frame<N>,
    {
        AddFrame {
            signal: self,
            frame,
        }
    }

    /// Creates a new [`Signal`] that yields each [`Frame`] of a [`Signal`]
    /// multiplied with a constant [`Frame`].
    fn mul_frame<F>(self, frame: F) -> MulFrame<Self, F, N>
    where
        Self: Sized,
        Self::Frame: Frame<N, Float = F>,
        F: Frame<N>,
    {
        MulFrame {
            signal: self,
            frame,
        }
    }

    /// Creates a new [`Signal`] that yields each [`Frame`] of a [`Signal`]
    /// with each channel summed with a constant [`Sample`].
    fn add_amp<X>(self, amp: X) -> AddAmp<Self, X, N>
    where
        Self: Sized,
        Self::Frame: Frame<N>,
        <Self::Frame as Frame<N>>::Sample: Sample<Signed = X>,
        X: Sample,
    {
        AddAmp { signal: self, amp }
    }

    /// Creates a new [`Signal`] that yields each [`Frame`] of a [`Signal`]
    /// with each channel multiplied with a constant [`Sample`].
    fn mul_amp<X>(self, amp: X) -> MulAmp<Self, X, N>
    where
        Self: Sized,
        Self::Frame: Frame<N>,
        <Self::Frame as Frame<N>>::Sample: Sample<Float = X>,
        X: Sample,
    {
        MulAmp { signal: self, amp }
    }

    /// Delays [`Self`] by a given number of frames. The delay is performed by
    /// yielding [`Frame::EQUILIBRIUM`] that number of times before continuing
    /// to yield frames from [`Self`].
    fn delay(self, n_frames: usize) -> Delay<Self, N>
    where
        Self: Sized,
    {
        Delay {
            signal: self,
            n_frames,
        }
    }

    /// Calls an inspection function on each [`Frame`] yielded by this
    /// [`Signal`], and then passes the [`Frame`] through.
    ///
    /// ```
    /// use sampara::{signal, Signal};
    ///
    /// fn main() {
    ///     let mut max: Option<i32> = None;
    ///     let mut signal = signal::from_frames(vec![2i32, 3, 1])
    ///         .inspect(|&f| {
    ///             if let Some(m) = max {
    ///                 max.replace(m.max(f));
    ///             } else {
    ///                 max = Some(f);
    ///             }
    ///         });
    ///
    ///     assert_eq!(signal.next(), Some(2));
    ///     assert_eq!(signal.next(), Some(3));
    ///     assert_eq!(signal.next(), Some(1));
    ///     assert_eq!(signal.next(), None);
    ///     assert_eq!(max, Some(3));
    /// }
    /// ```
    fn inspect<F>(self, func: F) -> Inspect<Self, F, N>
    where
        Self: Sized,
        F: FnMut(&Self::Frame),
    {
        Inspect { signal: self, func }
    }

    /// Returns a new [`Signal`] that yields only the first N [`Frame`]s of
    /// [`Self`].
    ///
    /// ```
    /// use sampara::{signal, Signal};
    ///
    /// fn main() {
    ///     let mut signal = signal::from_frames(0u8..=99)
    ///         .take(3);
    ///
    ///     assert_eq!(signal.next(), Some(0));
    ///     assert_eq!(signal.next(), Some(1));
    ///     assert_eq!(signal.next(), Some(2));
    ///     assert_eq!(signal.next(), None);
    /// }
    /// ```
    fn take(self, n: usize) -> Take<Self, N>
    where
        Self: Sized,
    {
        Take { signal: self, n }
    }

    /// Returns a new [`Signal`] that yields all the [`Frame`]s of [`Self`],
    /// and then [`Frame::EQUILIBRIUM`] until at least N total [`Frame`]s have
    /// been yielded.
    ///
    /// ```
    /// use sampara::{signal, Signal, Frame};
    ///
    /// fn main() {
    ///     let mut signal = signal::from_frames(vec![9, 8, 7])
    ///         .pad(4);
    ///
    ///     assert_eq!(signal.next(), Some(9));
    ///     assert_eq!(signal.next(), Some(8));
    ///     assert_eq!(signal.next(), Some(7));
    ///     assert_eq!(signal.next(), Some(Frame::EQUILIBRIUM));
    ///     assert_eq!(signal.next(), None);
    ///
    ///     // Yields the full original signal if padding is less than its
    ///     // length.
    ///     let mut signal = signal::from_frames(vec![9, 8, 7])
    ///         .pad(2);
    ///
    ///     assert_eq!(signal.next(), Some(9));
    ///     assert_eq!(signal.next(), Some(8));
    ///     assert_eq!(signal.next(), Some(7));
    ///     assert_eq!(signal.next(), None);
    /// }
    /// ```
    fn pad(self, n: usize) -> Pad<Self, N>
    where
        Self: Sized,
    {
        Pad { signal: self, n }
    }

    /// Returns a new [`Signal`] that yields every `n`th [`Frame`] from this
    /// [`Signal`]. The first [`Frame`] of this [`Signal`] is always returned.
    ///
    /// ```
    /// use sampara::{signal, Signal};
    ///
    /// fn main() {
    ///     let mut signal = signal::from_frames(0..=9);
    ///
    ///     let mut step_by = signal.step_by(2);
    ///     assert_eq!(step_by.next(), Some(0));
    ///     assert_eq!(step_by.next(), Some(2));
    ///     assert_eq!(step_by.next(), Some(4));
    ///     assert_eq!(step_by.next(), Some(6));
    ///     assert_eq!(step_by.next(), Some(8));
    ///     assert_eq!(step_by.next(), None);
    /// }
    /// ```
    ///
    /// This method pancis if `n == 0`.
    ///
    /// ```should_panic
    /// use sampara::{signal, Signal};
    ///
    /// fn main() {
    ///     let mut signal = signal::from_frames(0..=9);
    ///
    ///     let mut step_by = signal.step_by(0);
    ///     step_by.next();
    /// }
    /// ```
    fn step_by(self, step: usize) -> StepBy<Self, N>
    where
        Self: Sized,
    {
        StepBy::new(self, step)
    }

    /// Eagerly advances and discards `N` [`Frame`]s from [`Self`]. If there
    /// are fewer than `N` [`Frame`]s found, this will return `Err(X)`, where
    /// `X` is the number of [`Frame`]s actually advanced. Otherwise, returns
    /// `Ok(())`.
    ///
    /// ```
    /// use sampara::{signal, Signal};
    ///
    /// fn main() {
    ///     let mut signal = signal::from_frames(0u8..=9);
    ///
    ///     // Skip ahead 5 frames.
    ///     assert_eq!(signal.advance_by(5), Ok(()));
    ///
    ///     assert_eq!(signal.next(), Some(5));
    ///     assert_eq!(signal.next(), Some(6));
    ///
    ///     // Try to skip ahead 5 more frames.
    ///     assert_eq!(signal.advance_by(5), Err(3));
    ///
    ///     assert_eq!(signal.next(), None);
    /// }
    /// ```
    fn advance_by(&mut self, n: usize) -> Result<(), usize> {
        let mut left = n;
        while left > 0 {
            self.next().ok_or_else(|| n - left)?;
            left -= 1;
        }

        Ok(())
    }

    /// Converts this [`Signal`] into an [`Iterator`] yielding [`Frame`]s.
    ///
    /// ```
    /// use sampara::{signal, Signal};
    ///
    /// fn main() {
    ///     let signal = signal::from_frames(vec![2i32, 3, 1]).add_amp(5);
    ///     let iter = signal.into_iter();
    ///
    ///     assert_eq!(iter.collect::<Vec<_>>(), vec![7, 8, 6]);
    /// }
    /// ```
    // NOTE/TODO: This is a trait method on `Signal` as opposed to an impl of
    // `IntoIterator`, due to trait restrictions. We cannot have a blanket
    // `impl<S: Signal<N>, ...> IntoIterator for S`, since the `N` is
    // unconstrained. But, `Signal` requires `N` as a const generic input due
    // to `Frame` also requiring it. `Frame` uses `N` for defining fixed-size
    // array types in its methods. If associated consts could be used as const
    // generic bounds and/or fixed array sizes, then `Frame` (and thus
    // `Signal`) could just have `N` be an associated constant and drop the
    // const generic. Then, we could have a blanket impl of `IntoInterator`.
    // At that point, we could even do some specialization to make things even
    // more efficient!
    fn into_iter(self) -> IntoIter<Self, N>
    where
        Self: Sized,
    {
        IntoIter { signal: self }
    }

    /// Uses the [`Frame`]s of this [`Signal`] to fill a [`Buffer`]. If there
    /// are not enough [`Frame`]s to fill the [`Buffer`], returns [`Err`],
    /// containing the number of [`Frame`]s actaully read.
    ///
    /// ```
    /// use sampara::{signal, Signal};
    ///
    /// fn main() {
    ///     let mut signal = signal::from_frames(0..=9);
    ///
    ///     let mut buffer = [-1i8; 4];
    ///     assert_eq!(signal.fill_buffer(&mut buffer), Ok(()));
    ///     assert_eq!(buffer, [0, 1, 2, 3]);
    ///
    ///     let mut buffer = [-1i8; 8];
    ///     assert_eq!(signal.fill_buffer(&mut buffer), Err(6));
    ///     assert_eq!(buffer, [4, 5, 6, 7, 8, 9, -1, -1]);
    /// }
    /// ```
    fn fill_buffer<'a, B>(&mut self, buffer: &'a mut B) -> Result<(), usize>
    where
        B: Buffer<N, Frame = Self::Frame>,
    {
        for (num_filled, c) in buffer.as_mut().iter_mut().enumerate() {
            match self.next() {
                None => {
                    return Err(num_filled);
                }
                Some(frame) => {
                    *c = frame;
                }
            }
        }

        Ok(())
    }

    /// Creates a new [`Signal`] that passes the [`Frame`]s yielded from this
    /// [`Signal`] into a [`Processor`] that outputs [`Frame`]s, and yields the
    /// output [`Frame`]s.
    fn process<P, const NO: usize>(self, processor: P) -> Process<Self, P, N, NO>
    where
        Self: Sized,
        P: Processor<Input = Self::Frame>,
        P::Output: Frame<NO>,
    {
        Process {
            signal: self,
            processor,
        }
    }

    fn process_lazy<P, F, const NO: usize>(
        self,
        lazy_processor: P,
    ) -> ProcessLazy<Self, P, F, N, NO>
    where
        Self: Sized,
        P: Processor<Input = Self::Frame, Output = Option<F>>,
        F: Frame<NO>,
    {
        ProcessLazy {
            signal: self,
            lazy_processor,
        }
    }

    /// Creates a new [`Signal`] that passes the [`Frame`]s yielded from this
    /// [`Signal`] along with the [`Frame`]s yielded from another input
    /// [`Signal`] into a [`Combinator`], and yields the output [`Frame`]s.
    ///
    /// If one of the input [`Signal`]s finishes before the other, this new
    /// [`Signal`] will finish as well.
    fn combine<S, C, const NB: usize, const NO: usize>(
        self,
        other: S,
        combinator: C,
    ) -> Combine<Self, S, C, N, NB, NO>
    where
        Self: Sized,
        S: Signal<NB>,
        C: Combinator<InputL = Self::Frame, InputR = S::Frame>,
        C::Output: Frame<NO>,
    {
        Combine {
            signal_l: self,
            signal_r: other,
            combinator,
        }
    }

    fn combine_lazy<S, C, F, const NB: usize, const NO: usize>(
        self,
        other: S,
        lazy_combinator: C,
    ) -> CombineLazy<Self, S, C, F, N, NB, NO>
    where
        Self: Sized,
        S: Signal<NB>,
        C: Combinator<InputL = Self::Frame, InputR = S::Frame, Output = Option<F>>,
        F: Frame<NO>,
    {
        CombineLazy {
            signal_l: self,
            signal_r: other,
            lazy_combinator,
        }
    }

    /// Performs biquad filtering on this [`Signal`] and yields filtered
    /// [`Frame`]s in the same format as the original [`Signal`].
    ///
    /// ```
    /// use sampara::{signal, Signal};
    /// use sampara::biquad::Params;
    ///
    /// fn main() {
    ///     // Notch filter.
    ///     let params = Params::notch(0.25, 0.7071);
    ///
    ///     let input_signal = signal::from_frames(vec![
    ///          0.00000,  0.97553,  0.29389, -0.79389,
    ///         -0.47553,  0.50000,  0.47553, -0.20611,
    ///         -0.29389,  0.02447,  0.00000, -0.02447,
    ///          0.29389,  0.20611, -0.47553, -0.50000,
    ///     ]);
    ///
    ///     let expected = &[
    ///          0.000000000000000000,  0.571449973490183000,
    ///          0.172156092287300080,  0.008359170317441045,
    ///         -0.135938340413138700, -0.173590260270683420,
    ///          0.023322699278900627,  0.201938664486834900,
    ///          0.102400391831115600, -0.141048083352848520,
    ///         -0.189724745380021540,  0.024199368786658026,
    ///          0.204706829399554650,  0.102249983202951780,
    ///         -0.141523012483346670, -0.189698940039210730,
    ///     ];
    ///
    ///     let mut filtered_signal = input_signal.biquad(params);
    ///
    ///     let mut produced = vec![];
    ///     while let Some(filtered_frame) = filtered_signal.next() {
    ///         produced.push(filtered_frame);
    ///     }
    ///
    ///     assert_eq!(&produced, expected);
    /// }
    /// ```
    fn biquad(self, params: Params<<Self::Frame as Frame<N>>::Sample>) -> Biquad<Self, N>
    where
        Self: Sized,
        <Self::Frame as Frame<N>>::Sample: FloatSample,
    {
        Biquad {
            signal: self,
            filter: params.into(),
        }
    }

    /// Interpolates this [`Signal`] by yielding the [`Frame`]s at multiples of
    /// a given step size. If this step size falls in between two existing
    /// [`Frame`]s, the intermediate [`Frame`] is computed by using the given
    /// [`Interpolator`].
    ///
    /// An implicit equilibrium [`Frame`] is appended to the end of this
    /// [`Signal`] to allow for interpolating up to one [`Frame`] past the last.
    /// This only comes into effect if an interpolation value would fall past
    /// the last real [`Frame`] but before the next (non-existent) [`Frame`] is
    /// requested.
    ///
    /// This process is also known as "resampling".
    ///
    /// ```
    /// use sampara::{signal, Signal};
    /// use sampara::interpolate::Linear;
    ///
    /// fn main() {
    ///     let mut input_signal = signal::from_frames(vec![
    ///         [10, 10, 10],
    ///         [20, 30, 40],
    ///         [30, 50, 70],
    ///         [40, 70, 100],
    ///     ]);
    ///
    ///     // Initialize the interpolator with frames from the input signal.
    ///     let interpolator = Linear::new(
    ///         input_signal.next().unwrap(),
    ///         input_signal.next().unwrap(),
    ///     );
    ///     let mut interpolated = input_signal.interpolate(interpolator, 0.75);
    ///
    ///     assert_eq!(interpolated.next(), Some([10, 10, 10]));  // 0.00
    ///     assert_eq!(interpolated.next(), Some([17, 25, 32]));  // 0.75
    ///     assert_eq!(interpolated.next(), Some([25, 40, 55]));  // 1.50
    ///     assert_eq!(interpolated.next(), Some([32, 55, 77]));  // 2.25
    ///     assert_eq!(interpolated.next(), Some([40, 70, 100])); // 3.00
    ///     assert_eq!(interpolated.next(), Some([10, 17, 25]));  // 3.75
    ///     assert_eq!(interpolated.next(), None);                // 4.50
    /// }
    /// ```
    fn interpolate<I>(self, interpolator: I, step: f64) -> Interpolate<Self, I, N>
    where
        Self: Sized,
        I: Interpolator<N, Frame = Self::Frame>,
        <Self::Frame as Frame<N>>::Sample: Duplex<f64>,
    {
        Interpolate {
            signal: self,
            interpolator,
            interpolant: 0.0,
            step,
            end_padding: Some(Frame::EQUILIBRIUM),
        }
    }

    /// Feeds this [`Signal`] into a [`Calculator`], then returns the output of
    /// the consumed [`Frame`]s.
    fn calculate<C>(self, calculator: C) -> C::Output
    where
        Self: Sized,
        C: Calculator<Input = Self::Frame>,
    {
        let mut this = self;
        let mut calculator = calculator;

        while let Some(frame) = this.next() {
            calculator.push(frame);
        }

        calculator.calculate()
    }

    stats_moving_inject_signal_methods!();
    stats_cumulative_inject_signal_methods!();
}

impl<S, const N: usize> Signal<N> for &mut S
where
    S: Signal<N>,
{
    type Frame = S::Frame;

    fn next(&mut self) -> Option<Self::Frame> {
        (**self).next()
    }
}

// NOTE: Need to wait until `N` can be embedded as an associated constant,
//       which requires associated consts to be usable as generic array sizes.
// impl<S, const N: usize> IntoIterator for S
// where
//     S: Signal<N>,
// {
//     type Item = S::Frame;
//     type IntoIter: IntoIter<Self, N>;

//     fn into_iter(self) -> Self::IntoIter
//     {
//         IntoIter {
//             signal: self,
//         }
//     }
// }

/// Creates a new [`Signal`] where each [`Frame`] is yielded by calling a given
/// closure that produces a [`Option<Frame>`] for each iteration.
///
/// ```
/// use sampara::{signal, Signal};
///
/// fn main() {
///     let mut state = 1;
///     let mut signal = signal::from_fn(|| {
///         if state < 4 {
///             let frame = [state, state * 2, state * 3];
///             state += 1;
///             Some(frame)
///         }
///         else { None }
///     });
///
///     assert_eq!(signal.next(), Some([1, 2, 3]));
///     assert_eq!(signal.next(), Some([2, 4, 6]));
///     assert_eq!(signal.next(), Some([3, 6, 9]));
///     assert_eq!(signal.next(), None);
/// }
/// ```
pub fn from_fn<F, G, const N: usize>(gen_fn: G) -> FromFn<F, G, N>
where
    F: Frame<N>,
    G: FnMut() -> Option<F>,
{
    FromFn(gen_fn)
}

/// Creates a new [`Signal`] where each [`Frame`] is copied from a given
/// constant [`Frame`].
///
/// ```
/// use sampara::{signal, Signal};
///
/// fn main() {
///     let mut signal = signal::constant([1, 2, 3, 4]);
///
///     assert_eq!(signal.next(), Some([1, 2, 3, 4]));
///     assert_eq!(signal.next(), Some([1, 2, 3, 4]));
///     assert_eq!(signal.next(), Some([1, 2, 3, 4]));
///     assert_eq!(signal.next(), Some([1, 2, 3, 4]));
/// }
/// ```
pub fn constant<F, const N: usize>(frame: F) -> Constant<F, N>
where
    F: Frame<N>,
{
    Constant(frame)
}

/// Creates a new [`Signal`] that always yields [`Frame::EQUILIBRIUM`].
///
/// ```
/// use sampara::{signal, Signal};
///
/// fn main() {
///     let mut signal = signal::equilibrium();
///
///     assert_eq!(signal.next(), Some([0, 0]));
///     assert_eq!(signal.next(), Some([0, 0]));
///     assert_eq!(signal.next(), Some([0, 0]));
///     assert_eq!(signal.next(), Some([0, 0]));
/// }
/// ```
pub fn equilibrium<F, const N: usize>() -> Equilibrium<F, N>
where
    F: Frame<N>,
{
    Equilibrium(Default::default())
}

/// Creates an empty [`Signal`] that yields no [`Frame`]s.
///
/// ```
/// use sampara::{signal, Signal};
///
/// fn main() {
///     // Need to have redundant number of channels, until associated consts
///     // can be used as const generics.
///     let mut signal = signal::empty::<[i8; 2], 2>();
///
///     assert_eq!(signal.next(), None);
///     assert_eq!(signal.next(), None);
///     assert_eq!(signal.next(), None);
///     assert_eq!(signal.next(), None);
/// }
/// ```
pub fn empty<F, const N: usize>() -> Empty<F, N>
where
    F: Frame<N>,
{
    Empty(Default::default())
}

/// Creates a new [`Signal`] by wrapping an iterable that yields [`Frame`]s.
///
/// ```
/// use sampara::{signal, Signal};
///
/// fn main() {
///     let frames = vec![[0, 0], [16, -16], [32, -32]];
///     let mut signal = signal::from_frames(frames);
///
///     assert_eq!(signal.next(), Some([0, 0]));
///     assert_eq!(signal.next(), Some([16, -16]));
///     assert_eq!(signal.next(), Some([32, -32]));
///     assert_eq!(signal.next(), None);
/// }
/// ```
pub fn from_frames<I, const N: usize>(iter: I) -> FromFrames<I::IntoIter, N>
where
    I: IntoIterator,
    I::Item: Frame<N>,
{
    FromFrames(iter.into_iter())
}

/// Creates a new [`Signal`] by wrapping an iterable that yields [`Sample`]s.
/// These [`Sample`]s are assumed to be interleaved, and in channel order.
/// The resulting [`Signal`] will read these [`Sample`]s into [`Frame`]s of the
/// desired size, and yield them. Any trailing [`Sample`]s that do not fully
/// complete a [`Frame`] will be discarded.
///
/// ```
/// use sampara::{signal, Signal};
///
/// fn main() {
///     let samples = vec![1, 2, 3, 4, 5, 6, 7];
///     let mut signal = signal::from_samples(samples);
///
///     assert_eq!(signal.next(), Some([1, 2]));
///     assert_eq!(signal.next(), Some([3, 4]));
///     assert_eq!(signal.next(), Some([5, 6]));
///     // Not enough remaining samples for a full frame, so they are discarded.
///     assert_eq!(signal.next(), None);
/// }
/// ```
pub fn from_samples<F, I, const N: usize>(iter: I) -> FromSamples<F, I::IntoIter, N>
where
    F: Frame<N, Sample = I::Item>,
    I: IntoIterator,
    I::Item: Sample,
{
    FromSamples(iter.into_iter(), Default::default())
}
