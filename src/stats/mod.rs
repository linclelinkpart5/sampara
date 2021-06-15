use std::cmp::Ordering;
use std::collections::VecDeque;

use num_traits::Float;

use crate::{Frame, Sample, Processor};
use crate::buffer::{Fixed, Buffer};
use crate::sample::FloatSample;

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
    fn __from(buffer: B) -> Self {
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
    fn __empty(buffer: B) -> Self {
        let mut new = Self {
            window: Fixed::from(buffer),
            sum: Frame::EQUILIBRIUM,
        };

        new.__reset();

        new
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
}

macro_rules! gen_doc_comment {
    ($cls:ty, $text:expr, { $($test_stmt:expr),* $(,)? }) => {
        concat!(
            $text, "\n",
            "```\n",
            "use sampara::stats::", stringify!($cls), ";\n\n",
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

macro_rules! define__empty {
    ($cls:ident, $curr:expr) => {
        apply_doc_comment! {
            gen_doc_comment!(
                $cls,
                concat!(
                    "Similar to [`", stringify!($cls), "::from`], but treats the provided buffer as ",
                    "empty and fills it with [`Frame::EQUILIBRIUM`].",
                ),
                {
                    "// These values get zeroed out.",
                    concat!("let mut window = ", stringify!($cls), "::empty([[-1.0]; 4]);"),
                    concat!("assert_eq!(window.current(), ", stringify!($curr), ");"),
                }
            ),
            {
                #[inline]
                pub fn empty(buffer: B) -> Self {
                    Self(StatsInner::__empty(buffer))
                }
            }
        }
    }
}

macro_rules! define__from {
    ($cls:ident, $curr:expr) => {
        apply_doc_comment! {
            gen_doc_comment!(
                $cls,
                concat!(
                    "Creates a new [`", stringify!($cls), "`] using a given [`Buffer`] as a window. ",
                    "The provided buffer is assumed to be filled with the initial window buffer [`Frame`]s.",
                ),
                {
                    concat!("let mut window = ", stringify!($cls), "::from([[0.5], [0.5], [0.5], [0.5]]);\n"),
                    concat!("assert_eq!(window.current(), ", stringify!($curr), ");"),
                }
            ),
            {
                #[inline]
                fn from(buffer: B) -> Self {
                    Self(StatsInner::__from(buffer))
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
                    concat!("let mut window = ", stringify!($cls), "::from([[0.25], [0.75], [0.25], [0.75]]);"),
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
                    concat!("let mut window = ", stringify!($cls), "::empty([[-1.0]; 4]);"),
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
                    concat!("let mut window = ", stringify!($cls), "::empty([[-1.0]; 4]);"),
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
                    concat!("let window = ", stringify!($cls), "::empty([[0.0]; 99]);"),
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
                    concat!("let mut window = ", stringify!($cls), "::from([[0.0], [0.25], [0.50], [0.75]]);\n"),
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
                    concat!("let mut window = ", stringify!($cls), "::from([[0.0], [0.25], [0.50], [0.75]]);\n"),
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
                    concat!("let mut window = ", stringify!($cls), "::from([[0.0], [0.25], [0.50], [0.75]]);\n"),
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

macro_rules! calculator {
    (
        $cls:ident,
        $prose:literal,
        $is_sqrt:expr,
        $is_pow2:expr,
        {
            args_from => ( $($ta_from:expr),* ),
            args_empty => ( $($ta_empty:expr),* ),
            args_reset => ( $($ta_reset:expr),* ),
            args_fill => ( $($ta_fill:expr),* ),
            args_fill_with => ( $($ta_fill_with:expr),* ),
            args_advance => ( $($ta_advance:expr),* ),
            args_current => ( $($ta_current:expr),* ),
            args_process => ( $($ta_process:expr),* ),
        }
    ) => {
        apply_doc_comment! {
            concat!("Keeps a running ", $prose, " of a window of [`Frame`]s over time."),
            {
                #[derive(Clone)]
                pub struct $cls<F, B, const N: usize>(StatsInner<F, B, N, $is_sqrt, $is_pow2>)
                where
                    F: Frame<N>,
                    F::Sample: FloatSample,
                    B: Buffer<Item = F>,
                ;
            }
        }

        impl<F, B, const N: usize> $cls<F, B, N>
        where
            F: Frame<N>,
            F::Sample: FloatSample,
            B: Buffer<Item = F>,
        {
            define__empty!($cls, $($ta_empty),*);
            define__reset!($cls, $($ta_reset),*);
            define__fill!($cls, $($ta_fill),*);
            define__fill_with!($cls, $($ta_fill_with),*);
            define__len!($cls);
            define__advance!($cls, $prose, $($ta_advance),*);
            define__current!($cls, $prose, $($ta_current),*);
            define__process!($cls, $prose, $($ta_process),*);
        }

        impl<F, B, const N: usize> From<B> for $cls<F, B, N>
        where
            F: Frame<N>,
            F::Sample: FloatSample,
            B: Buffer<Item = F>,
        {
            define__from!($cls, $($ta_from),*);
        }

        // Forward `Processor::process` to `Self::process`.
        impl<F, B, const N: usize> Processor<N, N> for $cls<F, B, N>
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
    };
}

calculator!(Mean, "mean", NO_SQRT, NO_POW2, {
    args_from => ([0.5]),
    args_empty => ([0.0]),
    args_reset => ([0.5], [0.0]),
    args_fill => ([0.0], [0.5]),
    args_fill_with => ([0.0], [0.375]),
    args_advance => ([0.625], [0.8125], [0.9375], [1.0]),
    args_current => ([0.375]),
    args_process => ([0.625], [0.8125], [0.9375], [1.0]),
});

calculator!(Ms, "MS", NO_SQRT, DO_POW2, {
    args_from => ([0.25]),
    args_empty => ([0.0]),
    args_reset => ([0.3125], [0.0]),
    args_fill => ([0.0], [0.25]),
    args_fill_with => ([0.0], [0.21875]),
    args_advance => ([0.46875], [0.703125], [0.890625], [1.0]),
    args_current => ([0.21875]),
    args_process => ([0.46875], [0.703125], [0.890625], [1.0]),
});

calculator!(Rms, "RMS", DO_SQRT, DO_POW2, {
    args_from => ([0.5]),
    args_empty => ([0.0]),
    args_reset => ([0.5590169943749475], [0.0]),
    args_fill => ([0.0], [0.5]),
    args_fill_with => ([0.0], [0.46770717334674267]),
    args_advance => ([0.6846531968814576], [0.8385254915624212], [0.9437293044088437], [1.0]),
    args_current => ([0.46770717334674267]),
    args_process => ([0.6846531968814576], [0.8385254915624212], [0.9437293044088437], [1.0]),
});

const DO_MAX: bool = true;
const DO_MIN: bool = false;

#[derive(Clone)]
struct ExtremaState<S, const N: usize, const MAX: bool>
where
    S: FloatSample,
{
    states: [((S, usize), Option<(S, usize)>); N],
    curr_global_idx: usize,
}

impl<S, const N: usize, const MAX: bool> ExtremaState<S, N, MAX>
where
    S: FloatSample,
{
    fn from_array(array: [S; N]) -> Self {
        let states = array.map(|x| ((x, 0), None));

        ExtremaState {
            states,
            curr_global_idx: 1,
        }
    }

    fn process_array(&mut self, array: [S; N]) {
        let i = self.curr_global_idx;
        self.curr_global_idx += 1;

        // See if any frontiers need to be updated.
        self.states.each_mut().zip(array).map(|((border, opt_horizon), x)| {
            let (ext, pos) = border;

            // Check if the new value is a new border extrema.
            match (x.partial_cmp(ext), MAX) {
                // Do nothing.
                (None, _) | (Some(Ordering::Less), DO_MAX) | (Some(Ordering::Greater), DO_MIN) => {},

                // Reset the border index, blow away any existing horizon, and
                // return.
                _ => {
                    *border = (x, i);
                    *opt_horizon = None;
                    return;
                },
            }

            // Check if the new value is a new horizon extrema.
            if let Some((ext, _)) = opt_horizon {
                match (x.partial_cmp(ext), MAX) {
                    // Do nothing.
                    (None, _) | (Some(Ordering::Less), DO_MAX) | (Some(Ordering::Greater), DO_MIN) => {},

                    // Reset the horizon index and return.
                    _ => {
                        *opt_horizon = Some((x, i - *pos - 1));
                    }
                }
            }
            else {
                // Initialize the newly-cleared horizon with the current sample.
                *opt_horizon = Some((x, 0));
            }
        });
    }
}

type MinimumState<S, const N: usize> = ExtremaState<S, N, DO_MIN>;
type MaximumState<S, const N: usize> = ExtremaState<S, N, DO_MAX>;

#[derive(Clone)]
struct MinMaxInner<F, B, const N: usize, const MAX: bool>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    window: Fixed<B>,
    opt_state: Option<ExtremaState<F::Sample, N, MAX>>,
}

impl<F, B, const N: usize, const MAX: bool> MinMaxInner<F, B, N, MAX>
where
    F: Frame<N>,
    F::Sample: FloatSample,
    B: Buffer<Item = F>,
{
    #[inline]
    fn __from(buffer: B) -> Self {
        assert!(buffer.as_ref().len() > 0, "buffer length cannot be 0");

        let mut opt_state: Option<ExtremaState<F::Sample, N, MAX>> = None;

        // Pre-scan the starting window.
        for frame in buffer.as_ref().iter() {
            if let Some(state) = opt_state.as_mut() {
                // Process any new extremas.
                state.process_array(frame.into_array());
            } else {
                // This branch should only execute on the first frame.
                let states = frame.into_array().map(|x| ((x, 0), None));

                opt_state = Some(
                    ExtremaState {
                        states,
                        curr_global_idx: 0,
                    }
                );
            }
        }

        Self {
            window: Fixed::from(buffer),
            opt_state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimum_state() {
        let mut min_state = MinimumState::from_array([0.0; 4]);

        assert_eq!(min_state.states, [
            ((0.0, 0), None),
            ((0.0, 0), None),
            ((0.0, 0), None),
            ((0.0, 0), None),
        ]);
        assert_eq!(min_state.curr_global_idx, 1);

        min_state.process_array([-0.5, -0.25, 0.25, 0.5]);

        assert_eq!(min_state.states, [
            ((-0.5, 1), None),
            ((-0.25, 1), None),
            ((0.0, 0), Some((0.25, 0))),
            ((0.0, 0), Some((0.5, 0))),
        ]);
        assert_eq!(min_state.curr_global_idx, 2);
    }
}
