use std::cmp::Ordering;
use std::convert::TryFrom;

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

#[derive(Copy, Clone)]
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

#[derive(Clone)]
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
    fn extend(&mut self, xs: [S; N]) -> [Diff; N] {
        self.update::<true>(xs)
    }

    fn process(&mut self, xs: [S; N]) -> [Diff; N] {
        self.update::<false>(xs)
    }

    fn update<const EXT: bool>(&mut self, xs: [S; N]) -> [Diff; N] {
        // Convert from mutable array ref to an array of mutable refs.
        let frontiers = self.frontiers.each_mut();
        let horizons = self.horizons.each_mut();

        if EXT {
            // Increment the cursor position.
            self.cursor_pos += 1;
        }

        // Temp var, since closure captures `self` mutably.
        // TODO: Remove once Rust 2021 lands, which should alleviate this need.
        let cursor_pos = self.cursor_pos;

        // Channel index, only used when a promotion occurs.
        let mut channel_idx = 0;

        // Process each channel in lockstep.
        frontiers.zip(horizons).zip(xs).map(|((f, opt_h), x)| {
            // Capture current channel index and increment.
            let ch = channel_idx;
            channel_idx += 1;

            let (f_ext, f_pos) = f;

            if EXT {
                // When extending, there are four cases to handle:
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
            }
            else {
                // When processing, there are eight cases to handle:
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

                let is_f_pop = f_pos == &0;

                if surpasses::<_, MAX>(&x, f_ext) {
                    // Case [PFF].
                    // Case [PNF].
                    set_frontier(f, opt_h, x, cursor_pos)
                }
                else if let Some(h) = opt_h {
                    let (h_ext, _h_pos) = h;

                    if !is_f_pop {
                        *f_pos -= 1;
                    }

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

                            // Set the frontier to the current value and
                            // position of the horizon. Since the horizon
                            // position is an offset relative to the end of the
                            // frontier, this value will be the correct new
                            // frontier position.
                            *f = *h;
                            *opt_h = None;

                            todo!("HANDLE PROMOTION/SCOUTING");
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
            }
        })
    }
}

impl<F, const N: usize, const MAX: bool> TryFrom<&[F]> for ExtremaState<F::Sample, N, MAX>
where
    F: Frame<N>,
{
    type Error = ();

    fn try_from(window: &[F]) -> Result<Self, Self::Error> {
        let mut opt_ext_state: Option<Self> = None;

        for frame in window.iter() {
            let xs = frame.into_array();

            if let Some(ext_state) = opt_ext_state.as_mut() {
                ext_state.extend(xs);
            }
            else {
                // Initialize the extrema state.
                opt_ext_state = Some(
                    ExtremaState {
                        // The one and only array seen so far is the first
                        // frontier extrema for all channels by default, and
                        // has an offset of 0.
                        frontiers: xs.map(|x| (x, 0)),

                        // No horizon state yet.
                        horizons: [None; N],

                        cursor_pos: 0,
                    }
                );
            }
        }

        opt_ext_state.ok_or(())
    }
}

type MinimumState<S, const N: usize> = ExtremaState<S, N, DO_MIN>;
type MaximumState<S, const N: usize> = ExtremaState<S, N, DO_MAX>;

#[derive(Clone)]
struct MinMaxInner<F, B, const N: usize, const MAX: bool>
where
    F: Frame<N>,
    B: Buffer<Item = F>,
{
    window: Fixed<B>,
    ext_state: ExtremaState<F::Sample, N, MAX>,
}

impl<F, B, const N: usize, const MAX: bool> MinMaxInner<F, B, N, MAX>
where
    F: Frame<N>,
    B: Buffer<Item = F>,
{
    #[inline]
    fn __from(buffer: B) -> Self {
        let ext_state = ExtremaState::try_from(buffer.as_ref()).expect("buffer length cannot be 0");

        Self {
            window: Fixed::from(buffer),
            ext_state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUFFER_A: [[f32; 5]; 16] = [
        [0.5, 0.9, 0.4, 0.2, 0.4],
        [0.5, 0.3, 0.5, 0.5, 0.6],
        [0.6, 0.3, 0.5, 0.9, 0.6],
        [0.4, 0.4, 0.7, 0.6, 0.7],
        [0.4, 0.7, 0.3, 0.1, 0.6],
        [0.6, 0.3, 0.6, 0.8, 0.6],
        [0.4, 0.4, 0.5, 0.3, 0.2],
        [0.5, 0.0, 0.7, 0.0, 0.5],
        [0.1, 0.3, 0.6, 0.3, 0.4],
        [0.5, 0.1, 0.2, 0.8, 0.2],
        [0.3, 0.3, 0.3, 0.3, 0.3],
        [0.8, 0.3, 0.5, 0.7, 0.4],
        [0.5, 0.5, 0.3, 0.5, 0.6],
        [0.7, 0.5, 0.7, 0.2, 0.0],
        [0.4, 0.5, 0.6, 0.7, 0.8],
        [0.1, 0.2, 0.7, 0.3, 0.8],
    ];

    const BUFFER_B: [[f32; 5]; 16] = [
        [0.2, 0.4, 0.8, 0.6, 0.4],
        [0.7, 0.4, 0.4, 0.3, 0.4],
        [0.3, 0.1, 0.4, 0.6, 0.4],
        [0.6, 0.3, 0.4, 0.4, 0.7],
        [0.5, 0.4, 0.3, 0.7, 0.7],
        [0.8, 0.4, 0.0, 0.5, 0.3],
        [0.6, 0.2, 0.5, 0.2, 0.7],
        [0.7, 0.5, 0.5, 0.2, 0.5],
        [0.5, 0.4, 0.3, 0.7, 0.4],
        [0.7, 0.2, 0.5, 0.5, 0.4],
        [0.6, 0.6, 0.7, 0.4, 0.1],
        [0.7, 0.4, 0.3, 0.4, 0.2],
        [0.3, 0.4, 0.7, 0.2, 0.3],
        [0.2, 0.5, 0.3, 0.7, 0.3],
        [0.2, 0.6, 0.5, 0.4, 0.6],
        [0.9, 0.4, 0.5, 0.7, 0.0],
    ];

    const BUFFER_C: [[f32; 5]; 16] = [
        [0.1, 0.6, 0.2, 0.5, 0.6],
        [0.4, 0.6, 0.7, 0.1, 0.3],
        [0.7, 0.7, 0.2, 0.4, 0.5],
        [0.5, 0.2, 0.5, 0.8, 0.4],
        [0.6, 0.2, 0.4, 0.7, 0.5],
        [0.5, 0.6, 0.3, 0.2, 0.2],
        [0.4, 0.5, 0.5, 0.3, 0.5],
        [0.4, 0.4, 0.5, 0.5, 0.8],
        [0.4, 0.7, 0.8, 0.3, 0.8],
        [0.5, 0.8, 0.8, 0.8, 0.2],
        [0.4, 0.4, 0.6, 0.3, 0.1],
        [0.4, 0.4, 0.5, 0.3, 0.7],
        [0.8, 0.3, 0.5, 0.2, 0.5],
        [0.8, 0.2, 0.5, 0.5, 0.0],
        [0.4, 0.1, 0.4, 0.7, 0.2],
        [0.4, 0.4, 0.6, 0.5, 0.2],
    ];

    #[test]
    fn minimum_state() {
        let min_state = MinimumState::try_from(BUFFER_A.as_slice()).unwrap();

        assert_eq!(min_state.frontiers, [
            (0.1, 15),
            (0.0, 7),
            (0.2, 9),
            (0.0, 7),
            (0.0, 13),
        ]);

        assert_eq!(min_state.horizons, [
            None,
            Some((0.1, 1)),
            Some((0.3, 2)),
            Some((0.2, 5)),
            Some((0.8, 1)),
        ]);

        let min_state = MinimumState::try_from(BUFFER_B.as_slice()).unwrap();

        assert_eq!(min_state.frontiers, [
            (0.2, 14),
            (0.1, 2),
            (0.0, 5),
            (0.2, 12),
            (0.0, 15),
        ]);

        assert_eq!(min_state.horizons, [
            Some((0.9, 0)),
            Some((0.2, 6)),
            Some((0.3, 7)),
            Some((0.4, 1)),
            None,
        ]);

        let min_state = MinimumState::try_from(BUFFER_C.as_slice()).unwrap();

        assert_eq!(min_state.frontiers, [
            (0.1, 0),
            (0.1, 14),
            (0.2, 2),
            (0.1, 1),
            (0.0, 13),
        ]);

        assert_eq!(min_state.horizons, [
            Some((0.4, 14)),
            Some((0.4, 0)),
            Some((0.3, 2)),
            Some((0.2, 10)),
            Some((0.2, 1)),
        ]);
    }

    #[test]
    fn maximum_state() {
        let max_state = MaximumState::try_from(BUFFER_A.as_slice()).unwrap();

        assert_eq!(max_state.frontiers, [
            (0.8, 11),
            (0.9, 0),
            (0.7, 15),
            (0.9, 2),
            (0.8, 15),
        ]);

        assert_eq!(max_state.horizons, [
            Some((0.7, 1)),
            Some((0.7, 3)),
            None,
            Some((0.8, 6)),
            None,
        ]);

        let max_state = MaximumState::try_from(BUFFER_B.as_slice()).unwrap();

        assert_eq!(max_state.frontiers, [
            (0.9, 15),
            (0.6, 14),
            (0.8, 0),
            (0.7, 15),
            (0.7, 6),
        ]);

        assert_eq!(max_state.horizons, [
            None,
            Some((0.4, 0)),
            Some((0.7, 11)),
            None,
            Some((0.6, 7)),
        ]);

        let max_state = MaximumState::try_from(BUFFER_C.as_slice()).unwrap();

        assert_eq!(max_state.frontiers, [
            (0.8, 13),
            (0.8, 9),
            (0.8, 9),
            (0.8, 9),
            (0.8, 8),
        ]);

        assert_eq!(max_state.horizons, [
            Some((0.4, 1)),
            Some((0.4, 5)),
            Some((0.6, 5)),
            Some((0.7, 4)),
            Some((0.7, 2)),
        ]);
    }
}
