use std::cmp::Ordering;

use num_traits::Float;

use crate::{Frame, Sample, Processor};
use crate::buffer::{Fixed, Buffer};
use crate::sample::FloatSample;

const EMPTY_BUFFER_MSG: &'static str = "buffer cannot be empty";

const DO_SQRT: bool = true;
const NO_SQRT: bool = false;
const DO_POW2: bool = true;
const NO_POW2: bool = false;

/// Types that perform a calculation using a sliding window (ring buffer) of
/// input data.
pub trait SlidingCalculator<B, const N: usize>: From<B> + Processor<N, N, Input = B::Item, Output = B::Item>
where
    B: Buffer,
    B::Item: Frame<N>
{
    fn from_empty(buffer: B) -> Self;
    fn len(&self) -> usize;
    fn reset(&mut self);
    fn fill(&mut self, fill_val: B::Item);
    fn fill_with<M: FnMut() -> B::Item>(&mut self, fill_func: M);
    fn advance(&mut self, input: B::Item);
    fn current(&self) -> B::Item;
}

#[derive(Clone)]
struct SummageInner<B, const N: usize, const SQRT: bool, const POW2: bool>
where
    B: Buffer,
    B::Item: Frame<N>,
    <B::Item as Frame<N>>::Sample: FloatSample,
{
    window: Fixed<B>,
    sum: B::Item,
}

impl<B, const N: usize, const SQRT: bool, const POW2: bool> SummageInner<B, N, SQRT, POW2>
where
    B: Buffer,
    B::Item: Frame<N>,
    <B::Item as Frame<N>>::Sample: FloatSample,
{
    #[inline]
    fn __from(buffer: B) -> Self {
        let mut buffer = buffer;
        let mut sum = B::Item::EQUILIBRIUM;

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
    fn __from_empty(buffer: B) -> Self {
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
    fn __fill(&mut self, fill_val: B::Item) {
        let mut fill_val = fill_val;

        if POW2 {
            // Calculate the squared frame, as that is what will actually be
            // stored in the window.
            fill_val.transform(|x| x * x);
        }

        self.window.fill(fill_val);

        // Since the buffer is filled with a constant value, just multiply to
        // calculate the sum.
        let len_f: <B::Item as Frame<N>>::Sample = Sample::from_sample(self.__len() as f32);
        self.sum = fill_val.mul_amp(len_f);
    }

    #[inline]
    fn __fill_with<M>(&mut self, fill_func: M)
    where
        M: FnMut() -> B::Item,
    {
        let mut fill_func = fill_func;
        let mut sum = B::Item::EQUILIBRIUM;

        let prepped_fill_func = || {
            let mut f = fill_func();

            if POW2 {
                // Square the frame.
                f.transform(|x| x * x);
            }

            // Before yielding the frame, add it to the running sum.
            sum.add_assign_frame(f.into_signed_frame());

            f
        };

        self.window.fill_with(prepped_fill_func);
        self.sum = sum;
    }

    #[inline]
    fn __advance(&mut self, input: B::Item) {
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
    fn __current(&self) -> B::Item {
        let len_f = Sample::from_sample(self.__len() as f32);
        let mut ret: B::Item = self.sum.apply(|s| s / len_f);

        if SQRT {
            ret.transform(Float::sqrt);
        }

        ret
    }

    #[inline]
    fn __process(&mut self, input: B::Item) -> B::Item {
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

macro_rules! define__from {
    ($helper_cls:ident, $cls:ident, $curr:expr) => {
        apply_doc_comment! {
            gen_doc_comment!(
                $cls,
                concat!(
                    "Creates a new [`", stringify!($cls), "`] using a given [`Buffer`] as a window. ",
                    "The provided buffer is assumed to be filled with the initial window buffer [`Frame`]s.",
                ),
                {
                    concat!("let mut window = ", stringify!($cls), "::from([[0.5]; 4]);\n"),
                    concat!("assert_eq!(window.current(), ", stringify!($curr), ");"),
                }
            ),
            {
                #[inline]
                fn from(buffer: B) -> Self {
                    assert!(buffer.as_ref().len() > 0, "{}", EMPTY_BUFFER_MSG);
                    Self($helper_cls::__from(buffer))
                }
            }
        }
    }
}

macro_rules! define__from_empty {
    ($helper_cls:ident, $cls:ident, $curr:expr) => {
        apply_doc_comment! {
            gen_doc_comment!(
                $cls,
                concat!(
                    "Similar to [`", stringify!($cls), "::from`], but treats the provided buffer as ",
                    "empty and fills it with [`Frame::EQUILIBRIUM`].",
                ),
                {
                    "// These values get zeroed out.",
                    concat!("let mut window = ", stringify!($cls), "::from_empty([[-1.0]; 4]);"),
                    concat!("assert_eq!(window.current(), ", stringify!($curr), ");"),
                }
            ),
            {
                #[inline]
                pub fn from_empty(buffer: B) -> Self {
                    assert!(buffer.as_ref().len() > 0, "{}", EMPTY_BUFFER_MSG);
                    Self($helper_cls::__from_empty(buffer))
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
                    concat!("let mut window = ", stringify!($cls), "::from_empty([[-1.0]; 4]);"),
                    concat!("assert_eq!(window.current(), ", stringify!($curr), ");\n"),
                    concat!("window.fill([0.5]);"),
                    concat!("assert_eq!(window.current(), ", stringify!($after), ");"),
                }
            ),
            {
                #[inline]
                pub fn fill(&mut self, fill_val: B::Item) {
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
                    concat!("let mut window = ", stringify!($cls), "::from_empty([[-1.0]; 4]);"),
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
                    M: FnMut() -> B::Item,
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
                    concat!("let window = ", stringify!($cls), "::from_empty([[0.0]; 99]);"),
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
                pub fn advance(&mut self, input: B::Item) {
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
                pub fn current(&self) -> B::Item {
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
                pub fn process(&mut self, input: B::Item) -> B::Item {
                    self.0.__process(input)
                }
            }
        }
    }
}

macro_rules! calculator {
    (
        $helper_cls:ident,
        [ $( $const_gen_state:expr ),* ],
        $cls:ident,
        [ $( $sample_kind:ident )? ],
        $prose:literal,
        {
            args_from => ( $($ta_from:expr),* ),
            args_from_empty => ( $($ta_from_empty:expr),* ),
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
                pub struct $cls<B, const N: usize>($helper_cls<B, N, $( $const_gen_state ),* >)
                where
                    B: Buffer,
                    B::Item: Frame<N>,
                    $(<B::Item as Frame<N>>::Sample: $sample_kind,)?
                ;
            }
        }

        impl<B, const N: usize> $cls<B, N>
        where
            B: Buffer,
            B::Item: Frame<N>,
            $(<B::Item as Frame<N>>::Sample: $sample_kind,)?
        {
            define__from_empty!($helper_cls, $cls, $($ta_from_empty),*);
            define__reset!($cls, $($ta_reset),*);
            define__fill!($cls, $($ta_fill),*);
            define__fill_with!($cls, $($ta_fill_with),*);
            define__len!($cls);
            define__advance!($cls, $prose, $($ta_advance),*);
            define__current!($cls, $prose, $($ta_current),*);
            define__process!($cls, $prose, $($ta_process),*);
        }

        impl<B, const N: usize> From<B> for $cls<B, N>
        where
            B: Buffer,
            B::Item: Frame<N>,
            $(<B::Item as Frame<N>>::Sample: $sample_kind,)?
        {
            define__from!($helper_cls, $cls, $($ta_from),*);
        }

        // Implement `SlidingCalculator` and forward all methods to `Self`.
        impl<B, const N: usize> SlidingCalculator<B, N> for $cls<B, N>
        where
            B: Buffer,
            B::Item: Frame<N>,
            $(<B::Item as Frame<N>>::Sample: $sample_kind,)?
        {
            #[inline]
            fn from_empty(buffer: B) -> Self {
                Self::from_empty(buffer)
            }

            #[inline]
            fn len(&self) -> usize {
                self.len()
            }

            #[inline]
            fn reset(&mut self) {
                self.reset()
            }

            #[inline]
            fn fill(&mut self, fill_val: B::Item) {
                self.fill(fill_val)
            }

            #[inline]
            fn fill_with<M: FnMut() -> B::Item>(&mut self, fill_func: M) {
                self.fill_with(fill_func)
            }

            #[inline]
            fn advance(&mut self, input: B::Item) {
                self.advance(input)
            }

            #[inline]
            fn current(&self) -> B::Item {
                self.current()
            }
        }

        // Implement `Processor` and forward all methods to `Self`.
        impl<B, const N: usize> Processor<N, N> for $cls<B, N>
        where
            B: Buffer,
            B::Item: Frame<N>,
            $(<B::Item as Frame<N>>::Sample: $sample_kind,)?
        {
            type Input = B::Item;
            type Output = B::Item;

            #[inline]
            fn process(&mut self, input: Self::Input) -> Self::Output {
                self.process(input)
            }
        }
    };
}

calculator!(SummageInner, [NO_SQRT, NO_POW2], Mean, [FloatSample], "mean", {
    args_from => ([0.5]),
    args_from_empty => ([0.0]),
    args_reset => ([0.5], [0.0]),
    args_fill => ([0.0], [0.5]),
    args_fill_with => ([0.0], [0.375]),
    args_advance => ([0.625], [0.8125], [0.9375], [1.0]),
    args_current => ([0.375]),
    args_process => ([0.625], [0.8125], [0.9375], [1.0]),
});

calculator!(SummageInner, [NO_SQRT, DO_POW2], Ms, [FloatSample], "MS", {
    args_from => ([0.25]),
    args_from_empty => ([0.0]),
    args_reset => ([0.3125], [0.0]),
    args_fill => ([0.0], [0.25]),
    args_fill_with => ([0.0], [0.21875]),
    args_advance => ([0.46875], [0.703125], [0.890625], [1.0]),
    args_current => ([0.21875]),
    args_process => ([0.46875], [0.703125], [0.890625], [1.0]),
});

calculator!(SummageInner, [DO_SQRT, DO_POW2], Rms, [FloatSample], "RMS", {
    args_from => ([0.5]),
    args_from_empty => ([0.0]),
    args_reset => ([0.5590169943749475], [0.0]),
    args_fill => ([0.0], [0.5]),
    args_fill_with => ([0.0], [0.46770717334674267]),
    args_advance => ([0.6846531968814576], [0.8385254915624212], [0.9437293044088437], [1.0]),
    args_current => ([0.46770717334674267]),
    args_process => ([0.6846531968814576], [0.8385254915624212], [0.9437293044088437], [1.0]),
});

const DO_MAX: bool = true;
const DO_MIN: bool = false;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Diff {
    // The new value was not an extrema, and neither the frontier nor horizon
    // values were mutated (frontier positions may have decremented).
    NoChange,

    // The new value replaced the old frontier extrema.
    Frontier,

    // The new value replaced the old horizon extrema.
    Horizon,

    // The new value was used to initialize an empty horizon.
    HorizonInit,

    // Popping the oldest frame off caused the horizon to be promoted, so a new
    // horizon value will need to be scouted out.
    Promoted,
}

fn surpasses<S: Sample, const MAX: bool>(candidate: &S, target: &S) -> bool {
    match candidate.partial_cmp(&target) {
        // The new value does not surpass the target extrema.
        None => false,
        Some(Ordering::Less) if MAX => false,
        Some(Ordering::Greater) if !MAX => false,

        _ => true,
    }
}

fn set_frontier<S: Sample>(
    frontier: &mut (S, usize),
    opt_horizon: &mut Option<(S, usize)>,
    contender: S,
    cursor_pos: usize,
) -> Diff
{
    // Set the new frontier extrema and position to the contender value and the
    // cursor position, respectively.
    *frontier = (contender, cursor_pos);

    // Clear out the horizon.
    *opt_horizon = None;

    Diff::Frontier
}

fn set_horizon<S: Sample>(
    horizon: &mut (S, usize),
    contender: S,
    frontier_pos: usize,
    cursor_pos: usize,
) -> Diff
{
    // Set the new horizon extrema and position to the contender value and
    // the current frontier offset, respectively.
    *horizon = (contender, cursor_pos - frontier_pos - 1);

    Diff::Horizon
}

fn set_horizon_init<S: Sample>(
    opt_horizon: &mut Option<(S, usize)>,
    contender: S,
    frontier_pos: usize,
    cursor_pos: usize,
    expect_zero: bool,
) -> Diff
{
    let frontier_offset = cursor_pos - frontier_pos - 1;
    if expect_zero {
        assert_eq!(frontier_offset, 0);
    }

    *opt_horizon = Some((contender, frontier_offset));

    Diff::HorizonInit
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct ExtremaState<S, const N: usize, const MAX: bool>
where
    S: Sample,
{
    frontiers: [(S, usize); N],
    horizons: [Option<(S, usize)>; N],
    cursor_pos: usize,
}

impl<S, const N: usize, const MAX: bool> ExtremaState<S, N, MAX>
where
    S: Sample,
{
    fn push(&mut self, xs: [S; N]) -> [Diff; N] {
        // Convert from mutable array ref to an array of mutable refs.
        let frontiers = self.frontiers.each_mut();
        let horizons = self.horizons.each_mut();

        // Increment the cursor position.
        self.cursor_pos += 1;

        // Temp var, since closure captures `self` mutably.
        // TODO: Remove once Rust 2021 lands, which should alleviate this need.
        let cursor_pos = self.cursor_pos;

        // Process each channel in lockstep.
        frontiers.zip(horizons).zip(xs).map(|((f, opt_h), x)| {
            // When pushing, there are four cases to handle:
            //
            // * Finding a new frontier extrema [EF].
            //   [3 1 0] 4 >>> [3 1 0 4]
            //    ^ ^                 ^
            //
            // * Finding a new horizon extrema [EH].
            //   [3 1 0] 2 >>> [3 1 0 2]
            //    ^ ^           ^     ^
            //
            // * Finding a normal value [EN].
            //   [3 2 0] 1 >>> [3 2 0 1]
            //    ^ ^           ^ ^
            //
            // * Initializing a horizon [EI].
            //   [2 1 3] 2 >>> [2 1 3 2]
            //        ^             ^ ^

            let (f_ext, f_pos) = f;

            if surpasses::<_, MAX>(&x, f_ext) {
                // Case [EF].
                set_frontier(f, opt_h, x, cursor_pos)
            }
            else if let Some(h) = opt_h {
                let (h_ext, _h_pos) = h;

                if surpasses::<_, MAX>(&x, h_ext) {
                    // Case [EH].
                    set_horizon(h, x, *f_pos, cursor_pos)
                }
                else {
                    // Case [EN].
                    Diff::NoChange
                }
            }
            else {
                // Case [EI].
                set_horizon_init(opt_h, x, *f_pos, cursor_pos, true)
            }
        })
    }

    fn push_pop<B>(&mut self, xs: [S; N], window: &Fixed<B>) -> [Diff; N]
    where
        B: Buffer,
        B::Item: Frame<N, Sample = S>,
    {
        // Convert from mutable array ref to an array of mutable refs.
        let frontiers = self.frontiers.each_mut();
        let horizons = self.horizons.each_mut();

        // Temp var, since closure captures `self` mutably.
        // TODO: Remove once Rust 2021 lands, which should alleviate this need.
        let cursor_pos = self.cursor_pos;

        // Channel index, only used when a promotion occurs.
        let mut channel_idx = 0;

        // Process each channel in lockstep.
        frontiers.zip(horizons).zip(xs).map(|((f, opt_h), x)| {
            // When push-and-popping, there are eight cases to handle:
            //
            // * Popping the frontier, finding a new frontier extrema [PFF].
            //   [2 0 1 0] 3 >>> 2 [0 1 0 3]
            //    ^   ^                   ^
            //
            // * Popping the frontier, finding a new horizon extrema [PFH]. (!)
            //   [3 0 1 0] 2 >>> 3 [0 1 0 2]
            //    ^   ^                   ^
            //
            // * Popping the frontier, finding a normal value [PFN].
            //   [3 0 1 0] 0 >>> 3 [0 1 0 0]
            //    ^   ^               ^   ^
            //
            // * Popping the frontier, initializing a horizon [PFI]. (!!)
            //   0 0 0 [3] 0 >>> 0 0 0 3 [0]
            //          ^                 ^
            //
            // * Popping a non-frontier, finding a new frontier extrema [PNF].
            //   [0 2 0 1] 3 >>> 0 [2 0 1 3]
            //      ^   ^                 ^
            //
            // * Popping a non-frontier, finding a new horizon extrema [PNH].
            //   [0 3 0 1] 2 >>> 0 [3 0 1 2]
            //      ^   ^           ^     ^
            //
            // * Popping a non-frontier, finding a normal value [PNN].
            //   [0 3 0 1] 0 >>> 0 [3 0 1 0]
            //      ^   ^           ^   ^
            //
            // * Popping a non-frontier, initializing a horizon [PNI].
            //   [0 0 0 1] 0 >>> 0 [0 0 1 0]
            //          ^               ^ ^
            //
            // (!) The new horizon swoops in just in time to steal the
            // frontier promotion from the old horizon.
            // (!!) With our invariants, this case should only ever occur
            // with windows of length 1.

            // Capture current channel index and increment.
            let ch = channel_idx;
            channel_idx += 1;

            let (f_ext, f_pos) = f;

            let is_f_pop = f_pos == &0;
            if !is_f_pop {
                *f_pos -= 1;
            }

            if surpasses::<_, MAX>(&x, f_ext) {
                // Case [PFF].
                // Case [PNF].
                set_frontier(f, opt_h, x, cursor_pos)
            }
            else if let Some(h) = opt_h {
                let (h_ext, h_pos) = h;

                match (is_f_pop, surpasses::<_, MAX>(&x, h_ext)) {
                    (false, false) => {
                        // Case [PNN].
                        Diff::NoChange
                    },

                    (false, true) => {
                        // Case [PNH].
                        set_horizon(h, x, *f_pos, cursor_pos)
                    },

                    (true, false) => {
                        // Case [PFN].

                        // Set the frontier to the current value and position
                        // of the horizon ("promoting" the horizon). Since the
                        // horizon position is an offset relative to the end
                        // of the frontier, this value will be the correct new
                        // frontier position.
                        *f_ext = *h_ext;
                        *f_pos = *h_pos;
                        *opt_h = None;

                        // Search the window to try and find a new horizon.
                        // We need to do this here in order to strictly
                        // maintain the frontier-horizon invariant rules.
                        let mut w = window.iter();
                        let mut disc_h = None;

                        // Skip all of the items up to and including the new
                        // frontier postion.
                        w.nth(*f_pos).expect("frontier pos should always be [0, WIN_LEN).");

                        // Map the window iterator to extract only the current
                        // channel.
                        let w = w.map(|frame| frame.channel(ch).expect("ch index should always be [0, N)."));

                        for (horizon_offset, y) in w.enumerate() {
                            if let Some((disc_h_ext, _)) = disc_h.as_mut() {
                                if surpasses::<S, MAX>(y, disc_h_ext) {
                                    disc_h = Some((*y, horizon_offset))
                                }
                            }
                            else {
                                assert_eq!(horizon_offset, 0, "discovery horizon should only be `None` on first loop");
                                disc_h = Some((*y, horizon_offset))
                            }
                        }

                        // Note that this could still be `None`, but it should
                        // only ever occur with windows of length 1.
                        *opt_h = disc_h;

                        Diff::Promoted
                    },

                    (true, true) => {
                        // Case [PFH].

                        // The frontier is about to be popped off, and this
                        // new value arrives just in time to surpass the
                        // current horizon and snipe the promotion to
                        // frontier.
                        set_frontier(f, opt_h, x, cursor_pos)
                    },
                }
            }
            else {
                if is_f_pop {
                    // Case [PFI].
                    set_frontier(f, opt_h, x, cursor_pos)
                }
                else {
                    // Case [PNI].
                    set_horizon_init(opt_h, x, *f_pos, cursor_pos, true)
                }
            }
        })
    }
}

impl<S, const N: usize, const MAX: bool> From<[S; N]> for ExtremaState<S, N, MAX>
where
    S: Sample,
{
    fn from(xs: [S; N]) -> Self {
        // Treat this array as the genesis state.
        Self {
            // The one and only array seen so far is the first frontier
            // extrema for all channels by default, and has an offset of 0.
            frontiers: xs.map(|x| (x, 0)),

            // No horizon state yet.
            horizons: [None; N],

            cursor_pos: 0,
        }
    }
}

impl<S, const N: usize, const MAX: bool> Default for ExtremaState<S, N, MAX>
where
    S: Sample,
{
    fn default() -> Self {
        Self {
            frontiers: [(S::EQUILIBRIUM, 0); N],
            horizons: [None; N],
            cursor_pos: 0,
        }
    }
}

#[derive(Clone)]
struct MinMaxInner<B, const N: usize, const MAX: bool>
where
    B: Buffer,
    B::Item: Frame<N>,
{
    window: Fixed<B>,
    ext_state: ExtremaState<<B::Item as Frame<N>>::Sample, N, MAX>,
}

impl<B, const N: usize, const MAX: bool> MinMaxInner<B, N, MAX>
where
    B: Buffer,
    B::Item: Frame<N>,
{
    #[inline]
    fn __from(buffer: B) -> Self {
        let mut buf_iter = buffer.as_ref().iter();

        // SAFETY: This method should only ever be called immediately after
        //         a buffer length assertion.
        let xs = unsafe { buf_iter.next().unwrap_unchecked() }.into_array();

        let mut ext_state = ExtremaState::<_, N, MAX>::from(xs);

        for frame in buf_iter {
            ext_state.push(frame.into_array());
        }

        Self {
            window: Fixed::from(buffer),
            ext_state,
        }
    }

    #[inline]
    fn __from_empty(buffer: B) -> Self {
        // Create a dummy value, and then reset it.
        let mut new = Self {
            window: Fixed::from(buffer),
            ext_state: ExtremaState::default(),
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
        self.__fill(Frame::EQUILIBRIUM)
    }

    #[inline]
    fn __fill(&mut self, fill_val: B::Item) {
        // SAFETY: We ensure that this struct never gets created with a buffer
        //         length of 0, so this should never underflow.
        let f_pos = self.__len() - 1;

        self.window.fill(fill_val);
        self.ext_state = ExtremaState {
            frontiers: fill_val.into_array().map(|x| (x, f_pos)),
            horizons: [None; N],
            cursor_pos: f_pos,
        };
    }

    #[inline]
    fn __fill_with<M>(&mut self, fill_func: M)
    where
        M: FnMut() -> B::Item,
    {
        let mut fill_func = fill_func;

        let mut opt_ext_state: Option<ExtremaState<_, N, MAX>> = None;

        let prepped_fill_func = || {
            let f = fill_func();

            if let Some(ext_state) = opt_ext_state.as_mut() {
                ext_state.push(f.into_array());
            }
            else {
                opt_ext_state = Some(ExtremaState::from(f.into_array()));
            }

            f
        };

        self.window.fill_with(prepped_fill_func);

        // SAFETY: We ensure that this struct never gets created with a buffer
        //         length of 0, so the fill function is expected to execute at
        //         least once and create the state.
        self.ext_state = unsafe { opt_ext_state.unwrap_unchecked() };
    }

    #[inline]
    fn __advance(&mut self, input: B::Item) {
        self.window.push(input);
        self.ext_state.push_pop(input.into_array(), &self.window);
    }

    #[inline]
    fn __current(&self) -> B::Item {
        self.ext_state.frontiers.map(|(f_ext, _f_pos)| f_ext).into_frame()
    }

    #[inline]
    fn __process(&mut self, input: B::Item) -> B::Item {
        self.__advance(input);
        self.__current()
    }
}

calculator!(MinMaxInner, [DO_MIN], Min, [], "minimum", {
    args_from => ([0.5]),
    args_from_empty => ([0.0]),
    args_reset => ([0.25], [0.0]),
    args_fill => ([0.0], [0.5]),
    args_fill_with => ([0.0], [0.0]),
    args_advance => ([0.25], [0.50], [0.75], [1.0]),
    args_current => ([0.00]),
    args_process => ([0.25], [0.50], [0.75], [1.0]),
});

calculator!(MinMaxInner, [DO_MAX], Max, [], "maximum", {
    args_from => ([0.5]),
    args_from_empty => ([0.0]),
    args_reset => ([0.75], [0.0]),
    args_fill => ([0.0], [0.5]),
    args_fill_with => ([0.0], [0.75]),
    args_advance => ([1.0], [1.0], [1.0], [1.0]),
    args_current => ([0.75]),
    args_process => ([1.0], [1.0], [1.0], [1.0]),
});

#[cfg(test)]
mod tests {
    use super::*;

    use proptest::prelude::*;

    const N: usize = 16;

    fn arb_frame() -> impl Strategy<Value = [f32; N]> {
        prop::array::uniform16(any::<f32>())
    }

    fn arb_input_buffer() -> impl Strategy<Value = Vec<[f32; N]>> {
        prop::collection::vec(arb_frame(), 1..=8)
    }

    fn arb_input_feed() -> impl Strategy<Value = Vec<[f32; N]>> {
        prop::collection::vec(arb_frame(), 0..=32)
    }

    fn elem_minmax<I, const MAX: bool>(iter: I) -> [f32; N]
    where
        I: IntoIterator<Item = [f32; 16]>,
    {
        iter.into_iter().reduce(|sa, sb| {
            sa.zip(sb).map(|(a, b)| {
                if surpasses::<_, MAX>(&a, &b) { a }
                else { b }
            })
        }).unwrap()
    }

    proptest! {
        #[test]
        fn prop_min_from(in_buf in arb_input_buffer()) {
            let window = Min::from(in_buf.clone());

            let expected = elem_minmax::<_, DO_MIN>(in_buf);
            let produced = window.current();

            assert_eq!(expected, produced);
        }

        #[test]
        fn prop_min_from_empty(in_buf in arb_input_buffer()) {
            let buf_len = in_buf.len();
            let window = Min::from_empty(in_buf);

            // The min value should be the equilibrium frame.
            let expected = <[f32; N]>::EQUILIBRIUM;
            let produced = window.current();

            assert_eq!(expected, produced);

            // The index of the min value should be at the very end of the window.
            let expected = [buf_len - 1; N];
            let produced = window.0.ext_state.frontiers.map(|(_f_ext, f_pos)| f_pos);

            assert_eq!(expected, produced);
        }

        #[test]
        fn prop_min_process(in_buf in arb_input_buffer(), in_feed in arb_input_feed()) {
            let mut window = Min::from(in_buf.clone());
            let mut manual_window = Fixed::from(in_buf);

            for xs in in_feed {
                manual_window.push(xs);

                let expected = elem_minmax::<_, DO_MIN>(manual_window.iter().copied());
                let produced = window.process(xs);

                assert_eq!(expected, produced);
            }
        }
    }

    proptest! {
        #[test]
        fn prop_max_from(in_buf in arb_input_buffer()) {
            let window = Max::from(in_buf.clone());

            let expected = elem_minmax::<_, DO_MAX>(in_buf);
            let produced = window.current();

            assert_eq!(expected, produced);
        }

        #[test]
        fn prop_max_from_empty(in_buf in arb_input_buffer()) {
            let buf_len = in_buf.len();
            let window = Max::from_empty(in_buf);

            // The max value should be the equilibrium frame.
            let expected = <[f32; N]>::EQUILIBRIUM;
            let produced = window.current();

            assert_eq!(expected, produced);

            // The index of the max value should be at the very end of the window.
            let expected = [buf_len - 1; N];
            let produced = window.0.ext_state.frontiers.map(|(_f_ext, f_pos)| f_pos);

            assert_eq!(expected, produced);
        }

        #[test]
        fn prop_max_process(in_buf in arb_input_buffer(), in_feed in arb_input_feed()) {
            let mut window = Max::from(in_buf.clone());
            let mut manual_window = Fixed::from(in_buf);

            for xs in in_feed {
                manual_window.push(xs);

                let expected = elem_minmax::<_, DO_MAX>(manual_window.iter().copied());
                let produced = window.process(xs);

                assert_eq!(expected, produced);
            }
        }
    }
}
