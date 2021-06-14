use num_traits::Float;

use crate::{Frame, Sample, Processor, Signal};
use crate::buffer::{Fixed, Buffer};
use crate::sample::FloatSample;
use crate::signal::Process;

const DO_SQRT: bool = true;
const NO_SQRT: bool = false;
const DO_POW2: bool = true;
const NO_POW2: bool = false;

#[derive(Clone)]
struct StatsInner<F, B, const N: usize, const SQRT: bool, const POW2: bool>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    window: Fixed<B>,
    sum: F,
}

impl<F, B, const N: usize, const SQRT: bool, const POW2: bool> StatsInner<F, B, N, SQRT, POW2>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    #[inline]
    fn __from_full(buffer: B) -> Self {
        let mut buffer = buffer;
        let mut sum = F::EQUILIBRIUM;

        for frame in buffer.as_mut().iter_mut() {
            if POW2 {
                // Since the passed-in buffer has raw frames, square them
                // in-place.
                frame.transform(|x| x * x);
            }

            sum.add_assign_frame(frame.into_signed_frame());
        }

        Self {
            window: Fixed::from(buffer),
            sum,
        }
    }

    #[inline]
    fn __len(&self) -> usize {
        self.window.capacity()
    }

    #[inline]
    fn __reset(&mut self) {
        // ASSUME: All float samples have an equilibrium of 0. That way this
        // code as written works for any combo of (SQRT, POW2).
        self.window.fill(Frame::EQUILIBRIUM);
        self.sum = Frame::EQUILIBRIUM;
    }

    #[inline]
    fn __fill(&mut self, fill_val: F) {
        let mut fill_val = fill_val;

        if POW2 {
            // Calculate the squared frame, as that is what will actually be
            // stored in the window.
            fill_val.transform(|x| x * x);
        }

        self.window.fill(fill_val);

        // Since the buffer is filled with a constant value, just multiply to
        // calculate the sum.
        let len_f: F::Sample = Sample::from_sample(self.__len() as f32);
        self.sum = fill_val.mul_amp(len_f);
    }

    #[inline]
    fn __fill_with<M>(&mut self, fill_func: M)
    where
        M: FnMut() -> F,
    {
        let mut fill_func = fill_func;
        let mut sum = F::EQUILIBRIUM;

        let prepped_fill_func = || {
            let mut f = fill_func();

            if POW2 {
                // Square the frame.
                f.transform(|x| x * x);
            }

            // Before yielding the squared frame, add it to the running sum.
            sum.add_assign_frame(f.into_signed_frame());

            f
        };

        self.window.fill_with(prepped_fill_func);
        self.sum = sum;
    }

    #[inline]
    fn __advance(&mut self, input: F) {
        let mut input = input;

        if POW2 {
            // Calculate the square of the new frame and push onto the buffer.
            input.transform(|x| x * x);
        }

        let popped = self.window.push(input);

        // Add the new input and subtract the popped frame from the sum.
        self.sum
            .add_assign_frame(input.into_signed_frame())
            .sub_assign_frame(popped.into_signed_frame());

        if SQRT {
            // In case of floating point rounding errors, floor at equilibrium.
            self.sum.transform(|x| x.max(Sample::EQUILIBRIUM));
        }
    }

    #[inline]
    fn __current(&self) -> F {
        let len_f = Sample::from_sample(self.__len() as f32);
        let mut ret: F = self.sum.apply(|s| s / len_f);

        if SQRT {
            ret.transform(Float::sqrt);
        }

        ret
    }

    #[inline]
    fn __process(&mut self, input: F) -> F {
        self.__advance(input);
        self.__current()
    }

    #[inline]
    fn __from(buffer: B) -> Self {
        let mut new = Self {
            window: Fixed::from(buffer),
            sum: Frame::EQUILIBRIUM,
        };

        new.__reset();

        new
    }
}

macro_rules! gen_doc_comment {
    ($cls:ty, $text:expr, { $($test_stmt:expr),* $(,)? }) => {
        concat!(
            $text, "\n",
            "```\n",
            "use sampara::rms::", stringify!($cls), ";\n\n",
            "fn main() {\n",
            $(
                concat!("    ", $test_stmt, "\n"),
            )*
            "}\n",
            "```\n",
        )
    };
}

macro_rules! apply_doc_comment {
    ($doc_comment:expr, { $($tt:tt)* }) => {
        #[doc = $doc_comment]
        $($tt)*
    };
}

macro_rules! define__from_full {
    ($cls:ident, $curr:expr, $p1:expr, $p2:expr, $p3:expr, $p4:expr) => {
        apply_doc_comment! {
            gen_doc_comment!(
                $cls,
                concat!(
                    "Similar to [`", stringify!($cls), "::from`], but treats the
                    passed-in buffer as already filled with input [`Frame`]s."
                ),
                {
                    concat!("let mut window = ", stringify!($cls), "::from_full([[0.5], [0.5], [0.5], [0.5]]);\n"),
                    concat!("assert_eq!(window.current(), ", stringify!($curr), ");"),
                    concat!("assert_eq!(window.process([1.0]), ", stringify!($p1), ");"),
                    concat!("assert_eq!(window.process([1.0]), ", stringify!($p2), ");"),
                    concat!("assert_eq!(window.process([1.0]), ", stringify!($p3), ");"),
                    concat!("assert_eq!(window.process([1.0]), ", stringify!($p4), ");"),
                }
            ),
            {
                #[inline]
                pub fn from_full(buffer: B) -> Self {
                    Self(StatsInner::__from_full(buffer))
                }
            }
        }
    }
}

macro_rules! define__reset {
    ($cls:ident, $curr:expr, $after:expr) => {
        apply_doc_comment! {
            gen_doc_comment!(
                $cls,
                "Resets the window to its zeroed-out state.",
                {
                    concat!("let mut window = ", stringify!($cls), "::from_full([[0.25], [0.75], [0.25], [0.75]]);"),
                    concat!("assert_eq!(window.current(), ", stringify!($curr), ");\n"),
                    concat!("window.reset();"),
                    concat!("assert_eq!(window.current(), ", stringify!($after), ");"),
                }
            ),
            {
                #[inline]
                pub fn reset(&mut self) {
                    self.0.__reset()
                }
            }
        }
    }
}

macro_rules! define__fill {
    ($cls:ident, $curr:expr, $after:expr) => {
        apply_doc_comment! {
            gen_doc_comment!(
                $cls,
                "Fills the window with a single constant [`Frame`] value.",
                {
                    concat!("let mut window = ", stringify!($cls), "::from([[-1.0]; 4]);"),
                    concat!("assert_eq!(window.current(), ", stringify!($curr), ");\n"),
                    concat!("window.fill([0.5]);"),
                    concat!("assert_eq!(window.current(), ", stringify!($after), ");"),
                }
            ),
            {
                #[inline]
                pub fn fill(&mut self, fill_val: F) {
                    self.0.__fill(fill_val)
                }
            }
        }
    }
}

macro_rules! define__fill_with {
    ($cls:ident, $curr:expr, $after:expr) => {
        apply_doc_comment! {
            gen_doc_comment!(
                $cls,
                "Fills the window by repeatedly calling a closure that produces [`Frame`]s.",
                {
                    concat!("let mut window = ", stringify!($cls), "::from([[-1.0]; 4]);"),
                    concat!("assert_eq!(window.current(), ", stringify!($curr), ");\n"),
                    "let mut x = 1.0;",
                    "window.fill_with(|| {",
                    "    x -= 0.25;",
                    "    [x]",
                    "});",
                    concat!("assert_eq!(window.current(), ", stringify!($after), ");"),
                }
            ),
            {
                #[inline]
                pub fn fill_with<M>(&mut self, fill_func: M)
                where
                    M: FnMut() -> F,
                {
                    self.0.__fill_with(fill_func)
                }
            }
        }
    }
}

macro_rules! define__len {
    ($cls:ident) => {
        apply_doc_comment! {
            gen_doc_comment!(
                $cls,
                "Returns the length of the window.",
                {
                    concat!("let window = ", stringify!($cls), "::from([[0.0]; 99]);"),
                    "assert_eq!(window.len(), 99);",
                }
            ),
            {
                #[inline]
                pub fn len(&self) -> usize {
                    self.0.__len()
                }
            }
        }
    }
}

macro_rules! define__advance {
    ($cls:ident, $prose:literal, $p1:expr, $p2:expr, $p3:expr, $p4:expr) => {
        apply_doc_comment! {
            gen_doc_comment!(
                $cls,
                concat!(
                    "Advances the state of the window buffer by pushing in a new input [`Frame`]. ",
                    "The oldest frame will be popped off in order to accomodate the new one.\n\n",
                    "This method does not calculate the current ", $prose, " value, ",
                    "which can be more performant for workflows that process multiple frames in bulk ",
                    "and do not need the intermediate ", $prose, " values.",
                ),
                {
                    concat!("let mut window = ", stringify!($cls), "::from_full([[0.0], [0.25], [0.50], [0.75]]);\n"),
                    "window.advance([1.0]);",
                    concat!("assert_eq!(window.current(), ", stringify!($p1), ");"),
                    "window.advance([1.0]);",
                    concat!("assert_eq!(window.current(), ", stringify!($p2), ");"),
                    "window.advance([1.0]);",
                    concat!("assert_eq!(window.current(), ", stringify!($p3), ");"),
                    "window.advance([1.0]);",
                    concat!("assert_eq!(window.current(), ", stringify!($p4), ");"),
                }
            ),
            {
                #[inline]
                pub fn advance(&mut self, input: F) {
                    self.0.__advance(input)
                }
            }
        }
    }
}

macro_rules! define__current {
    ($cls:ident, $prose:literal, $curr:expr) => {
        apply_doc_comment! {
            gen_doc_comment!(
                $cls,
                concat!(
                    "Calculates the current ", $prose, " value using the current window contents.",
                ),
                {
                    concat!("let mut window = ", stringify!($cls), "::from_full([[0.0], [0.25], [0.50], [0.75]]);\n"),
                    concat!("assert_eq!(window.current(), ", stringify!($curr), ");"),
                }
            ),
            {
                #[inline]
                pub fn current(&self) -> F {
                    self.0.__current()
                }
            }
        }
    }
}

macro_rules! define__process {
    ($cls:ident, $prose:literal, $p1:expr, $p2:expr, $p3:expr, $p4:expr) => {
        apply_doc_comment! {
            gen_doc_comment!(
                $cls,
                concat!(
                    "Processes a new input frame by advancing the state of the window buffer ",
                    "and then calculating the current ", $prose, " value.\n\n",
                    "This is equivalent to a call to [`", stringify!($cls), "::advance`] followed ",
                    "by a call to [`", stringify!($cls), "::current`].",
                ),
                {
                    concat!("let mut window = ", stringify!($cls), "::from_full([[0.0], [0.25], [0.50], [0.75]]);\n"),
                    concat!("assert_eq!(window.process([1.0]), ", stringify!($p1), ");"),
                    concat!("assert_eq!(window.process([1.0]), ", stringify!($p2), ");"),
                    concat!("assert_eq!(window.process([1.0]), ", stringify!($p3), ");"),
                    concat!("assert_eq!(window.process([1.0]), ", stringify!($p4), ");"),
                }
            ),
            {
                #[inline]
                pub fn process(&mut self, input: F) -> F {
                    self.0.__process(input)
                }
            }
        }
    }
}

/// Keeps a running mean of a window of [`Frame`]s over time.
#[derive(Clone)]
pub struct Mean<F, B, const N: usize>(StatsInner<F, B, N, NO_SQRT, NO_POW2>)
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
;

impl<F, B, const N: usize> Mean<F, B, N>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    define__from_full!(Mean, [0.5], [0.625], [0.75], [0.875], [1.0]);

    define__reset!(Mean, [0.5], [0.0]);

    define__fill!(Mean, [0.0], [0.5]);

    define__fill_with!(Mean, [0.0], [0.375]);

    define__len!(Mean);

    define__advance!(Mean, "mean", [0.625], [0.8125], [0.9375], [1.0]);

    define__current!(Mean, "mean", [0.375]);

    define__process!(Mean, "mean", [0.625], [0.8125], [0.9375], [1.0]);
}

impl<F, B, const N: usize> From<B> for Mean<F, B, N>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    /// Creates a new [`Self`] using a given [`Buffer`] as a window.
    ///
    /// The contents of the buffer will be discarded and overwritten with
    /// equilibrium values.
    ///
    /// ```
    /// use sampara::rms::Mean;
    ///
    /// fn main() {
    ///     // These values get zeroed out.
    ///     let mut window = Mean::from([[-1.0]; 4]);
    ///     assert_eq!(window.current(), [0.0]);
    ///
    ///     assert_eq!(window.process([1.0]), [0.25]);
    ///     assert_eq!(window.process([1.0]), [0.5]);
    ///     assert_eq!(window.process([1.0]), [0.75]);
    ///     assert_eq!(window.process([1.0]), [1.0]);
    /// }
    /// ```
    #[inline]
    fn from(buffer: B) -> Self {
        Self(StatsInner::__from(buffer))
    }
}

impl<F, B, const N: usize> Processor<N, N> for Mean<F, B, N>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    type Input = F;
    type Output = F;

    #[inline]
    fn process(&mut self, input: Self::Input) -> Self::Output {
        self.process(input)
    }
}

/// Keeps a running MS (mean square) of a window of [`Frame`]s over time.
#[derive(Clone)]
pub struct Ms<F, B, const N: usize>(StatsInner<F, B, N, NO_SQRT, DO_POW2>)
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
;

impl<F, B, const N: usize> Ms<F, B, N>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    define__from_full!(Ms, [0.25], [0.4375], [0.6250], [0.8125], [1.0]);

    define__reset!(Ms, [0.3125], [0.0]);

    define__fill!(Ms, [0.0], [0.25]);

    define__fill_with!(Ms, [0.0], [0.21875]);

    define__len!(Ms);

    define__advance!(Ms, "MS", [0.46875], [0.703125], [0.890625], [1.0]);

    define__current!(Ms, "MS", [0.21875]);

    define__process!(Ms, "MS", [0.46875], [0.703125], [0.890625], [1.0]);
}

impl<F, B, const N: usize> From<B> for Ms<F, B, N>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    /// Creates a new [`Ms`] using a given [`Buffer`] as a window.
    ///
    /// The contents of the buffer will be discarded and overwritten with
    /// equilibrium values.
    ///
    /// ```
    /// use sampara::rms::Ms;
    ///
    /// fn main() {
    ///     // These values get zeroed out.
    ///     let mut ms = Ms::from([[-1.0]; 4]);
    ///     assert_eq!(ms.current(), [0.0]);
    ///
    ///     assert_eq!(ms.process([1.0]), [0.25]);
    ///     assert_eq!(ms.process([1.0]), [0.5]);
    ///     assert_eq!(ms.process([1.0]), [0.75]);
    ///     assert_eq!(ms.process([1.0]), [1.0]);
    /// }
    /// ```
    #[inline]
    fn from(buffer: B) -> Self {
        Self(StatsInner::__from(buffer))
    }
}

impl<F, B, const N: usize> Processor<N, N> for Ms<F, B, N>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    type Input = F;
    type Output = F;

    #[inline]
    fn process(&mut self, input: Self::Input) -> Self::Output {
        self.process(input)
    }
}

/// Keeps a running RMS (root mean square) of a window of [`Frame`]s over time.
#[derive(Clone)]
pub struct Rms<F, B, const N: usize>(StatsInner<F, B, N, DO_SQRT, DO_POW2>)
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
;

impl<F, B, const N: usize> Rms<F, B, N>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    define__from_full!(Rms, [0.5], [0.6614378277661477], [0.7905694150420949], [0.9013878188659973], [1.0]);

    define__reset!(Rms, [0.5590169943749475], [0.0]);

    define__fill!(Rms, [0.0], [0.5]);

    define__fill_with!(Rms, [0.0], [0.46770717334674267]);

    define__len!(Rms);

    define__advance!(Rms, "RMS", [0.6846531968814576], [0.8385254915624212], [0.9437293044088437], [1.0]);

    define__current!(Rms, "RMS", [0.46770717334674267]);

    define__process!(Rms, "RMS", [0.6846531968814576], [0.8385254915624212], [0.9437293044088437], [1.0]);
}

impl<F, B, const N: usize> From<B> for Rms<F, B, N>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    /// Creates a new [`Rms`] using a given [`Buffer`] as a window.
    ///
    /// The contents of the buffer will be discarded and overwritten with
    /// equilibrium values.
    ///
    /// ```
    /// use sampara::rms::Rms;
    ///
    /// fn main() {
    ///     // These values get zeroed out.
    ///     let mut rms = Rms::from([[-1.0]; 4]);
    ///     assert_eq!(rms.current(), [0.0]);
    ///
    ///     assert_eq!(rms.process([1.0]), [0.5]);
    ///     assert_eq!(rms.process([1.0]), [0.7071067811865476]);
    ///     assert_eq!(rms.process([1.0]), [0.8660254037844386]);
    ///     assert_eq!(rms.process([1.0]), [1.0]);
    /// }
    /// ```
    #[inline]
    fn from(buffer: B) -> Self {
        Self(StatsInner::__from(buffer))
    }
}

impl<F, B, const N: usize> Processor<N, N> for Rms<F, B, N>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    type Input = F;
    type Output = F;

    #[inline]
    fn process(&mut self, input: Self::Input) -> Self::Output {
        self.process(input)
    }
}

impl<S, B, const N: usize> Process<S, Ms<S::Frame, B, N>, N, N>
where
    S: Signal<N>,
    <S::Frame as Frame<N>>::Sample: FloatSample,
    B: Buffer<Item = S::Frame>,
{
    #[inline]
    pub fn current(&self) -> S::Frame {
        self.processor.current()
    }
}

impl<S, B, const N: usize> Process<S, Rms<S::Frame, B, N>, N, N>
where
    S: Signal<N>,
    <S::Frame as Frame<N>>::Sample: FloatSample,
    B: Buffer<Item = S::Frame>,
{
    #[inline]
    pub fn current(&self) -> S::Frame {
        self.processor.current()
    }
}
