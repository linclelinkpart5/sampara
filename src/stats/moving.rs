// LEARN: This is needed in order to make the generated injection macros
// visible in other modules in the crate.
#![macro_use]

use super::*;

use num_traits::Float;

use crate::buffer::{Buffer, Fixed};
use crate::sample::FloatSample;
use crate::{Frame, Processor, Sample, StatefulProcessor};

#[derive(Clone)]
struct SummageInner<B, const N: usize, const SQRT: bool, const POW2: bool>
where
    B: Buffer<N>,
    <B::Frame as Frame<N>>::Sample: FloatSample,
{
    window: Fixed<B, N>,
    sum: B::Frame,
}

impl<B, const N: usize, const SQRT: bool, const POW2: bool> SummageInner<B, N, SQRT, POW2>
where
    B: Buffer<N>,
    <B::Frame as Frame<N>>::Sample: FloatSample,
{
    #[inline]
    fn __from(buffer: B) -> Self {
        let mut buffer = buffer;
        let mut sum = B::Frame::EQUILIBRIUM;

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
    fn __fill(&mut self, fill_val: B::Frame) {
        let mut fill_val = fill_val;

        if POW2 {
            // Calculate the squared frame, as that is what will actually be
            // stored in the window.
            fill_val.transform(|x| x * x);
        }

        self.window.fill(fill_val);

        // Since the buffer is filled with a constant value, just multiply to
        // calculate the sum.
        let len_f: <B::Frame as Frame<N>>::Sample = Sample::from_sample(self.__len() as f32);
        self.sum = fill_val.mul_amp(len_f);
    }

    #[inline]
    fn __fill_with<M>(&mut self, fill_func: M)
    where
        M: FnMut() -> B::Frame,
    {
        let mut fill_func = fill_func;
        let mut sum = B::Frame::EQUILIBRIUM;

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
    fn __advance(&mut self, input: B::Frame) {
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
    fn __current(&self) -> B::Frame {
        let len_f = Sample::from_sample(self.__len() as f32);
        let mut ret: B::Frame = self.sum.map(|s| s / len_f);

        if SQRT {
            ret.transform(Float::sqrt);
        }

        ret
    }

    #[inline]
    fn __process(&mut self, input: B::Frame) -> B::Frame {
        self.__advance(input);
        self.__current()
    }
}

type MovingRmsInner<B, const N: usize> = SummageInner<B, N, DO_SQRT, DO_POW2>;
type MovingMsInner<B, const N: usize> = SummageInner<B, N, NO_SQRT, DO_POW2>;
type MovingMeanInner<B, const N: usize> = SummageInner<B, N, NO_SQRT, NO_POW2>;

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

fn set_frontier<S: Sample>(
    frontier: &mut (S, usize),
    opt_horizon: &mut Option<(S, usize)>,
    contender: S,
    cursor_pos: usize,
) -> Diff {
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
) -> Diff {
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
) -> Diff {
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
        let mut diffs = [Diff::NoChange; N];

        // Create the zipped iterator.
        let iter = self
            .frontiers
            .iter_mut()
            .zip(self.horizons.iter_mut())
            .zip(xs.iter())
            .zip(diffs.iter_mut());

        // Increment the cursor position.
        self.cursor_pos += 1;

        // Temp var, since closure captures `self` mutably.
        // TODO: Remove once Rust 2021 lands, which should alleviate this need.
        let cursor_pos = self.cursor_pos;

        // Process each channel in lockstep.
        for (((f, opt_h), x), diff) in iter {
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

            *diff = if surpasses::<_, MAX>(x, &&*f_ext) {
                // Case [EF].
                set_frontier(f, opt_h, *x, cursor_pos)
            } else if let Some(h) = opt_h {
                let (h_ext, _h_pos) = h;

                if surpasses::<_, MAX>(x, &&*h_ext) {
                    // Case [EH].
                    set_horizon(h, *x, *f_pos, cursor_pos)
                } else {
                    // Case [EN].
                    Diff::NoChange
                }
            } else {
                // Case [EI].
                set_horizon_init(opt_h, *x, *f_pos, cursor_pos, true)
            };
        }

        diffs
    }

    fn push_pop<B>(&mut self, xs: [S; N], window: &Fixed<B, N>) -> [Diff; N]
    where
        B: Buffer<N>,
        B::Frame: Frame<N, Sample = S>,
    {
        let mut diffs = [Diff::NoChange; N];

        // Create the zipped iterator.
        let iter = self
            .frontiers
            .iter_mut()
            .zip(self.horizons.iter_mut())
            .zip(xs.iter())
            .zip(diffs.iter_mut());

        // Temp var, since closure captures `self` mutably.
        // TODO: Remove once Rust 2021 lands, which should alleviate this need.
        let cursor_pos = self.cursor_pos;

        // Channel index, only used when a promotion occurs.
        let mut channel_idx = 0;

        // Process each channel in lockstep.
        for (((f, opt_h), x), diff) in iter {
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

            *diff = if surpasses::<_, MAX>(x, &&*f_ext) {
                // Case [PFF].
                // Case [PNF].
                set_frontier(f, opt_h, *x, cursor_pos)
            } else if let Some(h) = opt_h {
                let (h_ext, h_pos) = h;

                match (is_f_pop, surpasses::<_, MAX>(x, h_ext)) {
                    (false, false) => {
                        // Case [PNN].
                        Diff::NoChange
                    }

                    (false, true) => {
                        // Case [PNH].
                        set_horizon(h, *x, *f_pos, cursor_pos)
                    }

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
                        w.nth(*f_pos)
                            .expect("frontier pos should always be [0, WIN_LEN).");

                        // Map the window iterator to extract only the current
                        // channel.
                        let w = w.map(|frame| {
                            frame
                                .channel(ch)
                                .expect("ch index should always be [0, N).")
                        });

                        for (horizon_offset, y) in w.enumerate() {
                            if let Some((disc_h_ext, _)) = disc_h.as_mut() {
                                if surpasses::<S, MAX>(y, disc_h_ext) {
                                    disc_h = Some((*y, horizon_offset))
                                }
                            } else {
                                assert_eq!(
                                    horizon_offset, 0,
                                    "discovery horizon should only be `None` on first loop"
                                );
                                disc_h = Some((*y, horizon_offset))
                            }
                        }

                        // Note that this could still be `None`, but it should
                        // only ever occur with windows of length 1.
                        *opt_h = disc_h;

                        Diff::Promoted
                    }

                    (true, true) => {
                        // Case [PFH].

                        // The frontier is about to be popped off, and this
                        // new value arrives just in time to surpass the
                        // current horizon and snipe the promotion to
                        // frontier.
                        set_frontier(f, opt_h, *x, cursor_pos)
                    }
                }
            } else {
                if is_f_pop {
                    // Case [PFI].
                    set_frontier(f, opt_h, *x, cursor_pos)
                } else {
                    // Case [PNI].
                    set_horizon_init(opt_h, *x, *f_pos, cursor_pos, true)
                }
            };
        }

        diffs
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
struct ExtremaInner<B, const N: usize, const MAX: bool>
where
    B: Buffer<N>,
{
    window: Fixed<B, N>,
    ext_state: ExtremaState<<B::Frame as Frame<N>>::Sample, N, MAX>,
}

impl<B, const N: usize, const MAX: bool> ExtremaInner<B, N, MAX>
where
    B: Buffer<N>,
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
    fn __fill(&mut self, fill_val: B::Frame) {
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
        M: FnMut() -> B::Frame,
    {
        let mut fill_func = fill_func;

        let mut opt_ext_state: Option<ExtremaState<_, N, MAX>> = None;

        let prepped_fill_func = || {
            let f = fill_func();

            if let Some(ext_state) = opt_ext_state.as_mut() {
                ext_state.push(f.into_array());
            } else {
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
    fn __advance(&mut self, input: B::Frame) {
        self.window.push(input);
        self.ext_state.push_pop(input.into_array(), &self.window);
    }

    #[inline]
    fn __current(&self) -> B::Frame {
        self.ext_state
            .frontiers
            .map(|(f_ext, _f_pos)| f_ext)
            .into_frame()
    }

    #[inline]
    fn __process(&mut self, input: B::Frame) -> B::Frame {
        self.__advance(input);
        self.__current()
    }
}

type MovingMinInner<B, const N: usize> = ExtremaInner<B, N, DO_MIN>;
type MovingMaxInner<B, const N: usize> = ExtremaInner<B, N, DO_MAX>;

// This macro creates all calculators, helper classes, typedefs, and signal
// adaptors, as well as sub-macros for injecting the typedefs and signal
// methods into where they are needed.
macro_rules! master {
    {
        $({
            // Desired name for the public calculator class.
            class_name => $cls:ident,

            // Desired name for the public method on `Signal` that uses this
            // calculator.
            func_name => $func_name:ident,

            // Optional extra bounds on the `Sample` type for this new
            // calculator (e.g. `FloatSample`).
            sample_trait_bounds => [ $( $sample_kind:ident )? ],

            // A human-readable term for what this calculator calculates (e.g.
            // "RMS", "maximum", etc),
            description => $prose:literal,

            doctest_expected_vals => {
                from => ( $ta__from:expr ),
                from_empty => ( $ta__from_empty:expr ),
                reset => ( $ta__reset__before:expr, $ta__reset__after:expr ),
                fill => ( $ta__fill__before:expr, $ta__fill__after:expr ),
                fill_with => ( $ta__fill_with__before:expr, $ta__fill_with__after:expr ),
                advance => ( $ta__advance__p1:expr, $ta__advance__p2:expr, $ta__advance__p3:expr, $ta__advance__p4:expr ),
                current => ( $ta__current:expr ),
                process => ( $ta__process__p1:expr, $ta__process__p2:expr, $ta__process__p3:expr, $ta__process__p4:expr ),
            }
        }),+ $(,)?
    } => {
        paste::paste! {
            $(
                apply_doc_comment! {
                    concat!(
                        "Keeps a moving (aka \"rolling\" or \"sliding\") ",
                        $prose, " of a window of [`Frame`]s over time.",
                    ),
                    {
                        #[derive(Clone)]
                        pub struct $cls<B, const N: usize>([<$cls Inner>]<B, N>)
                        where
                            B: Buffer<N>,
                            $(<B::Frame as Frame<N>>::Sample: $sample_kind,)?
                        ;
                    }
                }

                impl<B, const N: usize> $cls<B, N>
                where
                    B: Buffer<N>,
                    $(<B::Frame as Frame<N>>::Sample: $sample_kind,)?
                {
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
                                concat!("assert_eq!(window.current(), ", stringify!($ta__from_empty), ");"),
                            }
                        ),
                        {
                            #[inline]
                            pub fn from_empty(buffer: B) -> Self {
                                assert!(buffer.as_ref().len() > 0, "{}", EMPTY_BUFFER_MSG);
                                Self([<$cls Inner>]::__from_empty(buffer))
                            }
                        }
                    }

                    apply_doc_comment! {
                        gen_doc_comment!(
                            $cls,
                            "Resets the window to its zeroed-out state.",
                            {
                                concat!("let mut window = ", stringify!($cls), "::from([[0.25], [0.75], [0.25], [0.75]]);"),
                                concat!("assert_eq!(window.current(), ", stringify!($ta__reset__before), ");\n"),
                                concat!("window.reset();"),
                                concat!("assert_eq!(window.current(), ", stringify!($ta__reset__after), ");"),
                            }
                        ),
                        {
                            #[inline]
                            pub fn reset(&mut self) {
                                self.0.__reset()
                            }
                        }
                    }

                    apply_doc_comment! {
                        gen_doc_comment!(
                            $cls,
                            "Fills the window with a single constant [`Frame`] value.",
                            {
                                concat!("let mut window = ", stringify!($cls), "::from_empty([[-1.0]; 4]);"),
                                concat!("assert_eq!(window.current(), ", stringify!($ta__fill__before), ");\n"),
                                concat!("window.fill([0.5]);"),
                                concat!("assert_eq!(window.current(), ", stringify!($ta__fill__after), ");"),
                            }
                        ),
                        {
                            #[inline]
                            pub fn fill(&mut self, fill_val: B::Frame) {
                                self.0.__fill(fill_val)
                            }
                        }
                    }

                    apply_doc_comment! {
                        gen_doc_comment!(
                            $cls,
                            "Fills the window by repeatedly calling a closure that produces [`Frame`]s.",
                            {
                                concat!("let mut window = ", stringify!($cls), "::from_empty([[-1.0]; 4]);"),
                                concat!("assert_eq!(window.current(), ", stringify!($ta__fill_with__before), ");\n"),
                                "let mut x = 1.0;",
                                "window.fill_with(|| {",
                                "    x -= 0.25;",
                                "    [x]",
                                "});",
                                concat!("assert_eq!(window.current(), ", stringify!($ta__fill_with__after), ");"),
                            }
                        ),
                        {
                            #[inline]
                            pub fn fill_with<M>(&mut self, fill_func: M)
                            where
                                M: FnMut() -> B::Frame,
                            {
                                self.0.__fill_with(fill_func)
                            }
                        }
                    }

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
                                concat!("assert_eq!(window.current(), ", stringify!($ta__advance__p1), ");"),
                                "window.advance([1.0]);",
                                concat!("assert_eq!(window.current(), ", stringify!($ta__advance__p2), ");"),
                                "window.advance([1.0]);",
                                concat!("assert_eq!(window.current(), ", stringify!($ta__advance__p3), ");"),
                                "window.advance([1.0]);",
                                concat!("assert_eq!(window.current(), ", stringify!($ta__advance__p4), ");"),
                            }
                        ),
                        {
                            #[inline]
                            pub fn advance(&mut self, input: B::Frame) {
                                self.0.__advance(input)
                            }
                        }
                    }

                    apply_doc_comment! {
                        gen_doc_comment!(
                            $cls,
                            concat!(
                                "Calculates the current ", $prose, " value using the current window contents.",
                            ),
                            {
                                concat!("let mut window = ", stringify!($cls), "::from([[0.0], [0.25], [0.50], [0.75]]);\n"),
                                concat!("assert_eq!(window.current(), ", stringify!($ta__current), ");"),
                            }
                        ),
                        {
                            #[inline]
                            pub fn current(&self) -> B::Frame {
                                self.0.__current()
                            }
                        }
                    }

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
                                concat!("assert_eq!(window.process([1.0]), ", stringify!($ta__process__p1), ");"),
                                concat!("assert_eq!(window.process([1.0]), ", stringify!($ta__process__p2), ");"),
                                concat!("assert_eq!(window.process([1.0]), ", stringify!($ta__process__p3), ");"),
                                concat!("assert_eq!(window.process([1.0]), ", stringify!($ta__process__p4), ");"),
                            }
                        ),
                        {
                            #[inline]
                            pub fn process(&mut self, input: B::Frame) -> B::Frame {
                                Processor::process(self, input)
                            }
                        }
                    }
                }

                impl<B, const N: usize> From<B> for $cls<B, N>
                where
                    B: Buffer<N>,
                    $(<B::Frame as Frame<N>>::Sample: $sample_kind,)?
                {
                    apply_doc_comment! {
                        gen_doc_comment!(
                            $cls,
                            concat!(
                                "Creates a new [`", stringify!($cls), "`] using a given [`Buffer`] as a window. ",
                                "The provided buffer is assumed to be filled with the initial window buffer [`Frame`]s.",
                            ),
                            {
                                concat!("let mut window = ", stringify!($cls), "::from([[0.5]; 4]);\n"),
                                concat!("assert_eq!(window.current(), ", stringify!($ta__from), ");"),
                            }
                        ),
                        {
                            #[inline]
                            fn from(buffer: B) -> Self {
                                assert!(buffer.as_ref().len() > 0, "{}", EMPTY_BUFFER_MSG);
                                Self([<$cls Inner>]::__from(buffer))
                            }
                        }
                    }
                }

                // Implement `StatefulProcessor` and forward all methods to `Self`.
                impl<B, const N: usize> StatefulProcessor for $cls<B, N>
                where
                    B: Buffer<N>,
                    $(<B::Frame as Frame<N>>::Sample: $sample_kind,)?
                {
                    type Input = B::Frame;
                    type Output = B::Frame;

                    /// Same as [`Self::advance`].
                    #[inline]
                    fn advance(&mut self, input: Self::Input) {
                        self.advance(input)
                    }

                    /// Same as [`Self::current`].
                    #[inline]
                    fn current(&self) -> Self::Output {
                        self.current()
                    }
                }

                #[derive(Clone)]
                enum [< Buffered $cls State >]<B, const N: usize>
                where
                    B: Buffer<N>,
                    $(<B::Frame as Frame<N>>::Sample: $sample_kind,)?
                {
                    Dummy,
                    Uninit(Fixed<B, N>),
                    Active($cls<B, N>),
                }

                impl<B, const N: usize> [< Buffered $cls State >]<B, N>
                where
                    B: Buffer<N>,
                    $(<B::Frame as Frame<N>>::Sample: $sample_kind,)?
                {
                    #[inline]
                    fn __promote_inner(self) -> Self {
                        match self {
                            Self::Dummy => self,
                            Self::Active(..) => self,
                            Self::Uninit(ring_buffer) => {
                                let buffer = ring_buffer.into_buffer();
                                let calc = $cls::from(buffer);
                                Self::Active(calc)
                            }
                        }
                    }

                    #[inline]
                    fn __promote(&mut self) {
                        // Swap `self` with a dummy value.
                        let mut snatched = Self::Dummy;
                        std::mem::swap(&mut snatched, self);

                        let new_state = snatched.__promote_inner();
                        *self = new_state;
                    }

                    #[inline]
                    fn __from(buffer: B) -> Self {
                        Self::Uninit(Fixed::from_offset(buffer, 0))
                    }

                    #[inline]
                    fn __from_full(buffer: B) -> Self {
                        let mut new = Self::__from(buffer);
                        new.__promote();
                        new
                    }

                    #[inline]
                    fn __reset_inner(self) -> Self {
                        let raw_buffer = match self {
                            Self::Dummy => {
                                return self;
                            },

                            Self::Active(calc) => calc.0.window.into_buffer(),

                            Self::Uninit(ring_buffer) => ring_buffer.into_buffer(),
                        };

                        Self::__from(raw_buffer)
                    }

                    #[inline]
                    fn __reset(&mut self) {
                        // Swap `self` with a dummy value.
                        let mut snatched = Self::Dummy;
                        std::mem::swap(&mut snatched, self);

                        let new_state = snatched.__reset_inner();
                        *self = new_state;
                    }

                    #[inline]
                    fn __fill(&mut self, fill_val: B::Frame) {
                        match self {
                            Self::Dummy => {},
                            Self::Uninit(ring_buffer) => {
                                // This fills the buffer correctly, starting at index 0.
                                ring_buffer.fill(fill_val);
                                self.__promote();
                            },
                            Self::Active(calc) => {
                                calc.fill(fill_val);
                            },
                        }
                    }

                    #[inline]
                    fn __fill_with<M>(&mut self, fill_func: M)
                    where
                        M: FnMut() -> B::Frame,
                    {
                        match self {
                            Self::Dummy => {},
                            Self::Uninit(ring_buffer) => {
                                // This fills the buffer correctly, starting at index 0.
                                ring_buffer.fill_with(fill_func);
                                self.__promote();
                            },
                            Self::Active(calc) => {
                                calc.fill_with(fill_func);
                            },
                        }
                    }

                    #[inline]
                    fn __is_active(&self) -> bool {
                        match self {
                            Self::Active(..) => true,
                            _ => false,
                        }
                    }

                    #[inline]
                    fn __advance(&mut self, input: B::Frame) {
                        match self {
                            Self::Active(calc) => {
                                calc.advance(input);
                            },

                            Self::Uninit(ring_buffer) => {
                                let (_, was_reset) = ring_buffer.push_with_flag(input);

                                if was_reset {
                                    // The buffer has been filled, promote.
                                    self.__promote();
                                }
                            },

                            Self::Dummy => {},
                        }
                    }

                    #[inline]
                    fn __current(&self) -> Option<B::Frame> {
                        match self {
                            Self::Active(calc) => Some(calc.current()),
                            Self::Uninit(..) => None,
                            Self::Dummy => None,
                        }
                    }
                }

                apply_doc_comment! {
                    concat!(
                        "Keeps a buffered moving (aka \"rolling\" or \"sliding\") ",
                        $prose, " of a window of [`Frame`]s over time.\n\n",
                        "This ", $prose, " calculation is \"buffered\" in the sense ",
                        "that the initial [`Buffer`] is assumed to be empty, ",
                        "which means that this will not start producing output ",
                        "frames until the buffer is filled."
                    ),
                    {
                        #[derive(Clone)]
                        pub struct [< Buffered $cls >]<B, const N: usize>([<Buffered $cls State>]<B, N>)
                        where
                            B: Buffer<N>,
                            $(<B::Frame as Frame<N>>::Sample: $sample_kind,)?
                        ;
                    }
                }

                impl<B, const N: usize> [< Buffered $cls >]<B, N>
                where
                    B: Buffer<N>,
                    $(<B::Frame as Frame<N>>::Sample: $sample_kind,)?
                {
                    apply_doc_comment! {
                        gen_doc_comment!(
                            [< Buffered $cls >],
                            concat!(
                                "Similar to [`", stringify!([< Buffered $cls >]), "::from`], ",
                                "but treats the provided buffer as already ",
                                "initialized.",
                            ),
                            {
                                concat!("let mut window = ", stringify!([< Buffered $cls >]), "::from_full([[0.5]; 4]);"),
                                concat!("assert_eq!(window.current(), Some(", stringify!($ta__from), "));"),
                            }
                        ),
                        {
                            #[inline]
                            pub fn from_full(buffer: B) -> Self {
                                Self([< Buffered $cls State >]::__from_full(buffer))
                            }
                        }
                    }

                    apply_doc_comment! {
                        gen_doc_comment!(
                            [< Buffered $cls >],
                            "Resets the window to its uninitialized state.",
                            {
                                concat!("let mut window = ", stringify!([< Buffered $cls >]), "::from_full([[-1.0]; 4]);"),
                                concat!("assert_eq!(window.is_active(), true);\n"),
                                concat!("window.reset();"),
                                concat!("assert_eq!(window.is_active(), false);"),
                            }
                        ),
                        {
                            #[inline]
                            pub fn reset(&mut self) {
                                self.0.__reset()
                            }
                        }
                    }

                    apply_doc_comment! {
                        gen_doc_comment!(
                            [< Buffered $cls >],
                            "Reinitializes the window with a single constant [`Frame`] value.",
                            {
                                concat!("let mut window = ", stringify!([< Buffered $cls >]), "::from([[-1.0]; 4]);"),
                                "assert_eq!(window.current(), None);\n",
                                "window.fill([0.5]);",
                                concat!("assert_eq!(window.current(), Some(", stringify!($ta__fill__after), "));"),
                            }
                        ),
                        {
                            #[inline]
                            pub fn fill(&mut self, fill_val: B::Frame) {
                                self.0.__fill(fill_val)
                            }
                        }
                    }

                    apply_doc_comment! {
                        gen_doc_comment!(
                            [< Buffered $cls >],
                            "Reinitializes the window by repeatedly calling a closure that produces [`Frame`]s.",
                            {
                                concat!("let mut window = ", stringify!([< Buffered $cls >]), "::from([[-1.0]; 4]);"),
                                "assert_eq!(window.current(), None);\n",
                                "let mut x = 1.0;",
                                "window.fill_with(|| {",
                                "    x -= 0.25;",
                                "    [x]",
                                "});",
                                concat!("assert_eq!(window.current(), Some(", stringify!($ta__fill_with__after), "));"),
                            }
                        ),
                        {
                            #[inline]
                            pub fn fill_with<M>(&mut self, fill_func: M)
                            where
                                M: FnMut() -> B::Frame,
                            {
                                self.0.__fill_with(fill_func)
                            }
                        }
                    }

                    apply_doc_comment! {
                        gen_doc_comment!(
                            [< Buffered $cls >],
                            "Returns `true` if this window is active (i.e. initialized), `false` otherwise.",
                            {
                                concat!("let mut window = ", stringify!([< Buffered $cls >]), "::from([[-1.0]; 4]);"),
                                "assert_eq!(window.is_active(), false);",
                                "window.fill([-1.0]);",
                                "assert_eq!(window.is_active(), true);",
                            }
                        ),
                        {
                            #[inline]
                            pub fn is_active(&self) -> bool {
                                self.0.__is_active()
                            }
                        }
                    }

                    apply_doc_comment! {
                        gen_doc_comment!(
                            [< Buffered $cls >],
                            concat!(
                                "Advances the state of the window buffer by pushing in a new input [`Frame`]. ",
                                "The oldest frame will be popped off in order to accomodate the new one.\n\n",
                                "This method does not calculate the current ", $prose, " value, ",
                                "which can be more performant for workflows that process multiple frames in bulk ",
                                "and do not need the intermediate ", $prose, " values.",
                            ),
                            {
                                concat!("let mut window = ", stringify!([< Buffered $cls >]), "::from([[-1.0]; 4]);\n"),
                                "window.advance([0.25]);",
                                "assert_eq!(window.current(), None);",
                                "window.advance([0.50]);",
                                "assert_eq!(window.current(), None);",
                                "window.advance([0.75]);",
                                "assert_eq!(window.current(), None);",
                                "window.advance([1.00]);",
                                concat!("assert_eq!(window.current(), Some(", stringify!($ta__advance__p1), "));"),
                            }
                        ),
                        {
                            #[inline]
                            pub fn advance(&mut self, input: B::Frame) {
                                self.0.__advance(input)
                            }
                        }
                    }

                    apply_doc_comment! {
                        gen_doc_comment!(
                            [< Buffered $cls >],
                            concat!(
                                "Calculates the current ", $prose, " value using the current ",
                                "window contents, if initialized. Otherwise, returns `None`.",
                            ),
                            {
                                concat!("let mut window = ", stringify!([< Buffered $cls >]), "::from_full([[0.0], [0.25], [0.50], [0.75]]);"),
                                concat!("assert_eq!(window.current(), Some(", stringify!($ta__current), "));\n\n"),
                                concat!("let mut window = ", stringify!([< Buffered $cls >]), "::from([[0.0]; 4]);"),
                                "assert_eq!(window.current(), None);",
                            }
                        ),
                        {
                            #[inline]
                            pub fn current(&self) -> Option<B::Frame> {
                                self.0.__current()
                            }
                        }
                    }

                    apply_doc_comment! {
                        gen_doc_comment!(
                            [< Buffered $cls >],
                            concat!(
                                "Processes a new input frame by advancing the state of the window buffer ",
                                "and then calculating the current ", $prose, " value.\n\n",
                                "This is equivalent to a call to [`", stringify!([< Buffered $cls >]), "::advance`] followed ",
                                "by a call to [`", stringify!([< Buffered $cls >]), "::current`].",
                            ),
                            {
                                concat!("let mut window = ", stringify!([< Buffered $cls >]), "::from([[-1.0]; 4]);\n"),
                                concat!("assert_eq!(window.process([0.25]), None);"),
                                concat!("assert_eq!(window.process([0.50]), None);"),
                                concat!("assert_eq!(window.process([0.75]), None);"),
                                concat!("assert_eq!(window.process([1.00]), Some(", stringify!($ta__process__p1), "));"),
                            }
                        ),
                        {
                            #[inline]
                            pub fn process(&mut self, input: B::Frame) -> Option<B::Frame> {
                                // NOTE: We delegate like this since we want to take
                                //       advantage of the `Processor` blanket impl.
                                Processor::process(self, input)
                            }
                        }
                    }
                }

                impl<B, const N: usize> From<B> for [< Buffered $cls >]<B, N>
                where
                    B: Buffer<N>,
                    $(<B::Frame as Frame<N>>::Sample: $sample_kind,)?
                {
                    apply_doc_comment! {
                        gen_doc_comment!(
                            [< Buffered $cls >],
                            concat!(
                                "Creates a new [`", stringify!([< Buffered $cls >]), "`] ",
                                "using a given [`Buffer`] as a window. The provided ",
                                "buffer is assumed to be uninitialized, and will ",
                                "have its contents overwritten.",
                            ),
                            {
                                concat!("let mut window = ", stringify!([< Buffered $cls >]), "::from([[-1.0]; 4]);\n"),
                                concat!("assert_eq!(window.current(), None);"),
                            }
                        ),
                        {
                            #[inline]
                            fn from(buffer: B) -> Self {
                                assert!(buffer.as_ref().len() > 0, "{}", EMPTY_BUFFER_MSG);
                                Self([< Buffered $cls State >]::__from(buffer))
                            }
                        }
                    }
                }

                // Implement `StatefulProcessor` and forward all methods to `Self`.
                impl<B, const N: usize> StatefulProcessor for [< Buffered $cls >]<B, N>
                where
                    B: Buffer<N>,
                    $(<B::Frame as Frame<N>>::Sample: $sample_kind,)?
                {
                    type Input = B::Frame;
                    type Output = Option<B::Frame>;

                    /// Same as [`Self::advance`].
                    #[inline]
                    fn advance(&mut self, input: Self::Input) {
                        self.advance(input)
                    }

                    /// Same as [`Self::current`].
                    #[inline]
                    fn current(&self) -> Self::Output {
                        self.current()
                    }
                }
            )+

            // This is a generated macro that injects adaptors types and typedefs.
            macro_rules! stats_moving_inject_signal_adaptors {
                () => {
                    $(
                        // NOTE: This is an adaptor type!
                        apply_doc_comment! {
                            concat!(
                                "A [`Signal`] that calculates a moving ",
                                $prose, " of a window of [`Frame`]s over time."
                            ),
                            {
                                pub struct $cls<S, B, const N: usize>(pub(crate) Process<S, crate::stats::$cls<B, N>, N, N>)
                                where
                                    S: Signal<N>,
                                    B: Buffer<N, Frame = S::Frame>,
                                    $(<B::Frame as Frame<N>>::Sample: $sample_kind,)?
                                ;
                            }
                        }

                        impl<S, B, const N: usize> Signal<N> for $cls<S, B, N>
                        where
                            S: Signal<N>,
                            B: Buffer<N, Frame = S::Frame>,
                            $(<B::Frame as Frame<N>>::Sample: $sample_kind,)?
                        {
                            type Frame = B::Frame;

                            fn next(&mut self) -> Option<Self::Frame> {
                                self.0.next()
                            }
                        }

                        apply_doc_comment! {
                            concat!(
                                "A [`Signal`] that calculates a buffered moving ",
                                $prose, " of a window of [`Frame`]s over time.\n\n",
                                "This signal adaptor is buffered in the sense that the ",
                                "initial window is treated as uninitialized: ",
                                "before yielding the first ", $prose, " value, ",
                                "the window is filled with [`Frame`]s from a ",
                                "source [`Signal`]. The newly-filled window then ",
                                "yields the first ", $prose, " value. ",
                            ),
                            {
                                pub struct [< Buffered $cls >]<S, B, const N: usize>(pub(crate) ProcessLazy<S, crate::stats::[< Buffered $cls >]<B, N>, S::Frame, N, N>)
                                where
                                    S: Signal<N>,
                                    B: Buffer<N, Frame = S::Frame>,
                                    $(<B::Frame as Frame<N>>::Sample: $sample_kind,)?
                                ;
                            }
                        }

                        impl<S, B, const N: usize> Signal<N> for [<Buffered $cls>]<S, B, N>
                        where
                            S: Signal<N>,
                            B: Buffer<N, Frame = S::Frame>,
                            $(<B::Frame as Frame<N>>::Sample: $sample_kind,)?
                        {
                            type Frame = B::Frame;

                            fn next(&mut self) -> Option<Self::Frame> {
                                self.0.next()
                            }
                        }
                    )+
                };
            }

            // This is a generated macro that injects methods into the `Signal`
            // trait definition.
            macro_rules! stats_moving_inject_signal_methods {
                () => {
                    $(
                        apply_doc_comment! {
                            concat!(
                                "Calculates a windowed ", $prose, " of this [`Signal`]. ",
                                "The given [`Buffer`] will be zeroed out, and its length will determine the ",
                                $prose, " window length.\n\n",
                                "For an input [`Signal`] of length `N`, this will produce a new [`Signal`] that also yields `N` [`Frame`]s.",
                            ),
                            {
                                fn $func_name<B>(self, window: B) -> $cls<Self, B, N>
                                where
                                    Self: Sized,
                                    $(<Self::Frame as Frame<N>>::Sample: $sample_kind,)?
                                    B: Buffer<N, Frame = Self::Frame>,
                                {
                                    let processor = crate::stats::$cls::from_empty(window);
                                    $cls(self.process(processor))
                                }
                            }
                        }

                        apply_doc_comment! {
                            concat!(
                                "Similar to [`Signal::", stringify!($func_name), "`], but treats the passed-in ",
                                "[`Buffer`] as already full and containing valid [`Frame`]s.\n\n",
                                "For an input [`Signal`] of length `N`, this will produce a new [`Signal`] that also yields `N` [`Frame`]s.",
                            ),
                            {
                                fn [< $func_name _padded >]<B>(self, window: B) -> $cls<Self, B, N>
                                where
                                    Self: Sized,
                                    $(<Self::Frame as Frame<N>>::Sample: $sample_kind,)?
                                    B: Buffer<N, Frame = Self::Frame>,
                                {
                                    let processor = crate::stats::$cls::from(window);
                                    $cls(self.process(processor))
                                }
                            }
                        }

                        apply_doc_comment! {
                            concat!(
                                "Similar to [`Signal::", stringify!($func_name), "`], but fills the ",
                                "given [`Buffer`] with input [`Frame`]s from the [`Signal`]. ",
                                "Upon filling, the first [`Frame`] will be yielded.\n\n",
                                "For an input [`Signal`] of length `N` and an input [`Buffer`] of ",
                                "length `B`, this will produce a new [`Signal`] that yields ",
                                "`N - B + 1` [`Frame`]s (or 0 if `N < B`).",
                            ),
                            {
                                fn [< buffered_ $func_name >]<B>(self, window: B) -> [<Buffered $cls>]<Self, B, N>
                                where
                                    Self: Sized,
                                    $(<Self::Frame as Frame<N>>::Sample: $sample_kind,)?
                                    B: Buffer<N, Frame = Self::Frame>,
                                {
                                    let lazy_processor = crate::stats::[< Buffered $cls >]::from(window);
                                    [< Buffered $cls >](self.process_lazy(lazy_processor))
                                }
                            }
                        }
                    )+
                };
            }
        }
    };
}

master! {
    {
        class_name => MovingRms,
        func_name => moving_rms,
        sample_trait_bounds => [FloatSample],
        description => "root mean square",

        doctest_expected_vals => {
            from => ([0.5]),
            from_empty => ([0.0]),
            reset => ([0.5590169943749475], [0.0]),
            fill => ([0.0], [0.5]),
            fill_with => ([0.0], [0.46770717334674267]),
            advance => ([0.6846531968814576], [0.8385254915624212], [0.9437293044088437], [1.0]),
            current => ([0.46770717334674267]),
            process => ([0.6846531968814576], [0.8385254915624212], [0.9437293044088437], [1.0]),
        }
    },
    {
        class_name => MovingMs,
        func_name => moving_ms,
        sample_trait_bounds => [FloatSample],
        description => "mean square",

        doctest_expected_vals => {
            from => ([0.25]),
            from_empty => ([0.0]),
            reset => ([0.3125], [0.0]),
            fill => ([0.0], [0.25]),
            fill_with => ([0.0], [0.21875]),
            advance => ([0.46875], [0.703125], [0.890625], [1.0]),
            current => ([0.21875]),
            process => ([0.46875], [0.703125], [0.890625], [1.0]),
        }
    },
    {
        class_name => MovingMean,
        func_name => moving_mean,
        sample_trait_bounds => [FloatSample],
        description => "mean",

        doctest_expected_vals => {
            from => ([0.5]),
            from_empty => ([0.0]),
            reset => ([0.5], [0.0]),
            fill => ([0.0], [0.5]),
            fill_with => ([0.0], [0.375]),
            advance => ([0.625], [0.8125], [0.9375], [1.0]),
            current => ([0.375]),
            process => ([0.625], [0.8125], [0.9375], [1.0]),
        }
    },
    {
        class_name => MovingMin,
        func_name => moving_min,
        sample_trait_bounds => [],
        description => "minimum",

        doctest_expected_vals => {
            from => ([0.5]),
            from_empty => ([0.0]),
            reset => ([0.25], [0.0]),
            fill => ([0.0], [0.5]),
            fill_with => ([0.0], [0.0]),
            advance => ([0.25], [0.50], [0.75], [1.0]),
            current => ([0.00]),
            process => ([0.25], [0.50], [0.75], [1.0]),
        }
    },
    {
        class_name => MovingMax,
        func_name => moving_max,
        sample_trait_bounds => [],
        description => "maximum",

        doctest_expected_vals => {
            from => ([0.5]),
            from_empty => ([0.0]),
            reset => ([0.75], [0.0]),
            fill => ([0.0], [0.5]),
            fill_with => ([0.0], [0.75]),
            advance => ([1.0], [1.0], [1.0], [1.0]),
            current => ([0.75]),
            process => ([1.0], [1.0], [1.0], [1.0]),
        }
    },
}

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
        iter.into_iter()
            .reduce(|sa, sb| {
                let mut ret = [0.0f32; 16];

                for ((a, b), r) in sa.into_iter().zip(sb).zip(ret.iter_mut()) {
                    *r = if surpasses::<_, MAX>(&a, &b) { a } else { b };
                }

                ret
            })
            .unwrap()
    }

    proptest! {
        #[test]
        fn prop_min_from(in_buf in arb_input_buffer()) {
            let window = MovingMin::from(in_buf.clone());

            let expected = elem_minmax::<_, DO_MIN>(in_buf);
            let produced = window.current();

            assert_eq!(expected, produced);
        }

        #[test]
        fn prop_min_from_empty(in_buf in arb_input_buffer()) {
            let buf_len = in_buf.len();
            let window = MovingMin::from_empty(in_buf);

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
            let mut window = MovingMin::from(in_buf.clone());
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
            let window = MovingMax::from(in_buf.clone());

            let expected = elem_minmax::<_, DO_MAX>(in_buf);
            let produced = window.current();

            assert_eq!(expected, produced);
        }

        #[test]
        fn prop_max_from_empty(in_buf in arb_input_buffer()) {
            let buf_len = in_buf.len();
            let window = MovingMax::from_empty(in_buf);

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
            let mut window = MovingMax::from(in_buf.clone());
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
