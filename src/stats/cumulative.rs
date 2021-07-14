// LEARN: This is needed in order to make the generated injection macros
// visible in other modules in the crate.
#![macro_use]

use super::*;

use num_traits::{Float, NumCast};

use crate::{Frame, Sample, Processor};
use crate::sample::FloatSample;

#[derive(Clone)]
struct SummageInner<F, const N: usize, const SQRT: bool, const POW2: bool>
where
    F: Frame<N>,
    F::Sample: FloatSample,
{
    avg: F,
    count: u64,
}

impl<F, const N: usize, const SQRT: bool, const POW2: bool> SummageInner<F, N, SQRT, POW2>
where
    F: Frame<N>,
    F::Sample: FloatSample,
{
    #[inline]
    fn __is_active(&self) -> bool {
        self.count > 0
    }

    #[inline]
    fn __advance(&mut self, input: F) {
        let mut input = input;

        if POW2 {
            // Calculate the square of the new frame and push onto the buffer.
            input.transform(|x| x * x);
        }

        if self.count <= 0 {
            self.avg = input;
            self.count = 1;
        }
        else {
            self.count += 1;
            let c = <F::Sample as NumCast>::from(self.count).unwrap();
            self.avg.zip_transform(input, |a, x| {
                let mut new_a = a + (x - a) / c;
                if SQRT {
                    // In case of floating point rounding errors, floor at equilibrium.
                    new_a = new_a.max(Sample::EQUILIBRIUM);
                }
                new_a
            });
        }
    }

    #[inline(always)]
    fn __current_unchecked(&self) -> F {
        if SQRT { self.avg.apply(Float::sqrt) }
        else { self.avg }
    }

    #[inline]
    fn __default() -> Self {
        Self {
            avg: Frame::EQUILIBRIUM,
            count: 0,
        }
    }
}

type CumulativeRmsInner<F, const N: usize> = SummageInner<F, N, DO_SQRT, DO_POW2>;
type CumulativeMsInner<F, const N: usize> = SummageInner<F, N, NO_SQRT, DO_POW2>;
type CumulativeMeanInner<F, const N: usize> = SummageInner<F, N, NO_SQRT, NO_POW2>;

#[derive(Clone)]
struct ExtremaInner<F, const N: usize, const MAX: bool>
where
    F: Frame<N>,
{
    extrema: F,
    is_active: bool,
}

impl<F, const N: usize, const MAX: bool> ExtremaInner<F, N, MAX>
where
    F: Frame<N>,
{
    #[inline]
    fn __is_active(&self) -> bool {
        self.is_active
    }

    #[inline]
    fn __advance(&mut self, input: F) {
        if !self.is_active {
            self.extrema = input;
            self.is_active = true;
        }
        else {
            self.extrema.zip_transform(input, |e, x| {
                if crate::stats::surpasses::<_, MAX>(&x, &e) {
                    x
                }
                else {
                    e
                }
            });
        }
    }

    #[inline(always)]
    fn __current_unchecked(&self) -> F {
        self.extrema
    }

    #[inline]
    fn __default() -> Self {
        Self {
            extrema: Frame::EQUILIBRIUM,
            is_active: false,
        }
    }
}

type CumulativeMinInner<F, const N: usize> = ExtremaInner<F, N, DO_MIN>;
type CumulativeMaxInner<F, const N: usize> = ExtremaInner<F, N, DO_MAX>;

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
                reset => ( $ta__reset__before:expr ),
                advance => ( $ta__advance__p1:expr, $ta__advance__p2:expr, $ta__advance__p3:expr, $ta__advance__p4:expr ),
                current => ( $ta__current:expr ),
                try_current => ( $ta__try_current:expr ),
                process => ( $ta__process__p1:expr, $ta__process__p2:expr, $ta__process__p3:expr, $ta__process__p4:expr ),
            }
        }),+ $(,)?
    } => {
        paste::paste! {
            $(
                apply_doc_comment! {
                    concat!("Keeps a cumulative ", $prose, " of one or more [`Frame`]s over time."),
                    {
                        #[derive(Clone)]
                        pub struct $cls<F, const N: usize>([<$cls Inner>]<F, N>)
                        where
                            F: Frame<N>,
                            $(F::Sample: $sample_kind,)?
                        ;
                    }
                }

                impl<F, const N: usize> $cls<F, N>
                where
                    F: Frame<N>,
                    $(F::Sample: $sample_kind,)?
                {
                    apply_doc_comment! {
                        gen_doc_comment!(
                            $cls,
                            concat!(
                                "Resets this cumulative ", $prose, " to its initial empty state.",
                            ),
                            {
                                concat!("let mut calc = ", stringify!($cls), "::from([-0.5]);"),
                                concat!("assert_eq!(calc.current(), ", stringify!($ta__reset__before), ");\n"),
                                concat!("calc.reset();"),
                                concat!("assert_eq!(calc.try_current(), None);"),
                            }
                        ),
                        {
                            #[inline]
                            pub fn reset(&mut self) {
                                *self = Self::default();
                            }
                        }
                    }

                    apply_doc_comment! {
                        gen_doc_comment!(
                            $cls,
                            concat!(
                                "Returns true if this cumulative ", $prose, " is active (has ",
                                "processed at least one frame of data).",
                            ),
                            {
                                concat!("let mut calc = ", stringify!($cls), "::default();"),
                                concat!("assert_eq!(calc.is_active(), false);\n"),
                                concat!("calc.advance([0.5]);"),
                                concat!("assert_eq!(calc.is_active(), true);"),
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
                            $cls,
                            concat!(
                                "Advances the state of the cumulative ", $prose, " by pushing in a ",
                                "new input [`Frame`].\n\n",
                                "This method does not calculate the current ", $prose, " value, ",
                                "which can be more performant for workflows that process multiple frames in bulk ",
                                "and do not need the intermediate ", $prose, " values.",
                            ),
                            {
                                concat!("let mut calc = ", stringify!($cls), "::default();\n"),
                                "calc.advance([0.0]);",
                                concat!("assert_eq!(calc.current(), ", stringify!($ta__advance__p1), ");"),
                                "calc.advance([0.5]);",
                                concat!("assert_eq!(calc.current(), ", stringify!($ta__advance__p2), ");"),
                                "calc.advance([1.0]);",
                                concat!("assert_eq!(calc.current(), ", stringify!($ta__advance__p3), ");"),
                                "calc.advance([-0.5]);",
                                concat!("assert_eq!(calc.current(), ", stringify!($ta__advance__p4), ");"),
                            }
                        ),
                        {
                            #[inline]
                            pub fn advance(&mut self, input: F) {
                                self.0.__advance(input)
                            }
                        }
                    }

                    apply_doc_comment! {
                        gen_doc_comment!(
                            $cls,
                            // TODO: Add doctest for panic case.
                            concat!(
                                "Calculates the current cumulative ", $prose, " value. Panics if this ",
                                "calculator has not yet processed any frames (i.e. [`Self::is_active`] ",
                                "is `false`)."
                            ),
                            {
                                concat!("let mut calc = ", stringify!($cls), "::from([-0.5]);\n"),
                                concat!("assert_eq!(calc.current(), ", stringify!($ta__current), ");"),
                            }
                        ),
                        {
                            #[inline]
                            pub fn current(&self) -> F {
                                self.try_current().expect(ZERO_FRAMES_MSG)
                            }
                        }
                    }

                    apply_doc_comment! {
                        gen_doc_comment!(
                            $cls,
                            concat!(
                                "Calculates the current cumulative ", $prose, " value of this ",
                                "calculator if it is active. Otherwise, returns `None`"
                            ),
                            {
                                concat!("let mut calc = ", stringify!($cls), "::default();\n"),
                                "assert_eq!(calc.try_current(), None);",
                                "calc.advance([-0.5]);",
                                concat!("assert_eq!(calc.try_current(), Some(", stringify!($ta__try_current), "));"),
                            }
                        ),
                        {
                            #[inline]
                            pub fn try_current(&self) -> Option<F> {
                                if self.is_active() {
                                    Some(self.0.__current_unchecked())
                                }
                                else {
                                    None
                                }
                            }
                        }
                    }

                    apply_doc_comment! {
                        gen_doc_comment!(
                            $cls,
                            concat!(
                                "Processes a new input frame by advancing the cumulative ", $prose,
                                "state, and then calculating the current ", $prose, " value.\n\n",
                                "This is equivalent to a call to [`Self::advance`] followed by a ",
                                "call to [`Self::current`].",
                            ),
                            {
                                concat!("let mut calc = ", stringify!($cls), "::default();\n"),
                                concat!("assert_eq!(calc.process([0.0]), ", stringify!($ta__process__p1), ");"),
                                concat!("assert_eq!(calc.process([0.5]), ", stringify!($ta__process__p2), ");"),
                                concat!("assert_eq!(calc.process([1.0]), ", stringify!($ta__process__p3), ");"),
                                concat!("assert_eq!(calc.process([-0.5]), ", stringify!($ta__process__p4), ");"),
                            }
                        ),
                        {
                            #[inline]
                            pub fn process(&mut self, input: F) -> F {
                                self.advance(input);
                                self.0.__current_unchecked()
                            }
                        }
                    }
                }

                impl<F, const N: usize> From<F> for $cls<F, N>
                where
                    F: Frame<N>,
                    $(F::Sample: $sample_kind,)?
                {
                    apply_doc_comment! {
                        gen_doc_comment!(
                            $cls,
                            concat!(
                                "Creates a new [`", stringify!($cls), "`] using a given [`Frame`] as ",
                                "the first input point.",
                            ),
                            {
                                concat!("let mut calc = ", stringify!($cls), "::from([-0.5]);\n"),
                                concat!("assert_eq!(calc.current(), ", stringify!($ta__from), ");"),
                            }
                        ),
                        {
                            fn from(frame: F) -> Self {
                                let mut new = Self::default();
                                new.advance(frame);
                                new
                            }
                        }
                    }
                }

                impl<F, const N: usize> Default for $cls<F, N>
                where
                    F: Frame<N>,
                    $(F::Sample: $sample_kind,)?
                {
                    apply_doc_comment! {
                        gen_doc_comment!(
                            $cls,
                            concat!(
                                "Creates a new [`", stringify!($cls), "`] with an empty input state.",
                            ),
                            {
                                concat!("let mut calc = ", stringify!($cls), "::<f32, 1>::default();\n"),
                                concat!("assert_eq!(calc.try_current(), None);"),
                            }
                        ),
                        {
                            fn default() -> Self {
                                Self([<$cls Inner>]::__default())
                            }
                        }
                    }
                }

                impl<F, const N: usize> Processor<N, N> for $cls<F, N>
                where
                    F: Frame<N>,
                    $(F::Sample: $sample_kind,)?
                {
                    type Input = F;
                    type Output = F;

                    #[inline]
                    fn process(&mut self, input: Self::Input) -> Self::Output {
                        self.process(input)
                    }
                }
            )+

            // This is a generated macro that injects adaptors types and typedefs.
            macro_rules! stats_cumulative_inject_signal_adaptors {
                () => {
                    $(
                        // NOTE: This is an adaptor type!
                        apply_doc_comment! {
                            concat!(
                                "A [`Signal`] that calculates a cumulative ", $prose, " of one or more [`Frame`]s."
                            ),
                            {
                                pub struct $cls<S, const N: usize>(pub(crate) Process<S, crate::stats::$cls<S::Frame, N>, N, N>)
                                where
                                    S: Signal<N>,
                                    $(<S::Frame as Frame<N>>::Sample: $sample_kind,)?
                                ;
                            }
                        }

                        impl<S, const N: usize> Signal<N> for $cls<S, N>
                        where
                            S: Signal<N>,
                            $(<S::Frame as Frame<N>>::Sample: $sample_kind,)?
                        {
                            type Frame = S::Frame;

                            fn next(&mut self) -> Option<Self::Frame> {
                                self.0.next()
                            }
                        }
                    )+
                };
            }

            // This is a generated macro that injects methods into the `Signal`
            // trait definition.
            macro_rules! stats_cumulative_inject_signal_methods {
                () => {
                    $(
                        apply_doc_comment! {
                            concat!(
                                "Calculates a cumulative ", $prose, " of this [`Signal`]. The first ",
                                "[`Frame`] will set the initial state of the calculator.\n\n",
                                "For an input [`Signal`] of length `N`, this will produce a new ",
                                "[`Signal`] that also yields `N` [`Frame`]s.",
                            ),
                            {
                                fn $func_name(self) -> $cls<Self, N>
                                where
                                    Self: Sized,
                                    $(<Self::Frame as Frame<N>>::Sample: $sample_kind,)?
                                {
                                    let processor = crate::stats::$cls::default();
                                    $cls(self.process(processor))
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
        class_name => CumulativeRms,
        func_name => cumulative_rms,
        sample_trait_bounds => [FloatSample],
        description => "root mean square",

        doctest_expected_vals => {
            from => ([0.5]),
            reset => ([0.5]),
            advance => ([0.0], [0.3535533905932738], [0.6454972243679028], [0.6123724356957945]),
            current => ([0.5]),
            try_current => ([0.5]),
            process => ([0.0], [0.3535533905932738], [0.6454972243679028], [0.6123724356957945]),
        }
    },
    {
        class_name => CumulativeMs,
        func_name => cumulative_ms,
        sample_trait_bounds => [FloatSample],
        description => "mean square",

        doctest_expected_vals => {
            from => ([0.25]),
            reset => ([0.25]),
            advance => ([0.0], [0.125], [0.4166666666666667], [0.375]),
            current => ([0.25]),
            try_current => ([0.25]),
            process => ([0.0], [0.125], [0.4166666666666667], [0.375]),
        }
    },
    {
        class_name => CumulativeMean,
        func_name => cumulative_mean,
        sample_trait_bounds => [FloatSample],
        description => "mean",

        doctest_expected_vals => {
            from => ([-0.5]),
            reset => ([-0.5]),
            advance => ([0.0], [0.25], [0.5], [0.25]),
            current => ([-0.5]),
            try_current => ([-0.5]),
            process => ([0.0], [0.25], [0.5], [0.25]),
        }
    },
    {
        class_name => CumulativeMin,
        func_name => cumulative_min,
        sample_trait_bounds => [FloatSample],
        description => "minimum",

        doctest_expected_vals => {
            from => ([-0.5]),
            reset => ([-0.5]),
            advance => ([0.0], [0.0], [0.0], [-0.5]),
            current => ([-0.5]),
            try_current => ([-0.5]),
            process => ([0.0], [0.0], [0.0], [-0.5]),
        }
    },
    {
        class_name => CumulativeMax,
        func_name => cumulative_max,
        sample_trait_bounds => [FloatSample],
        description => "maximum",

        doctest_expected_vals => {
            from => ([-0.5]),
            reset => ([-0.5]),
            advance => ([0.0], [0.5], [1.0], [1.0]),
            current => ([-0.5]),
            try_current => ([-0.5]),
            process => ([0.0], [0.5], [1.0], [1.0]),
        }
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    use proptest::prelude::*;
    use approx::assert_relative_eq;

    const N: usize = 16;

    fn arb_frame() -> impl Strategy<Value = [f32; N]> {
        prop::array::uniform16(-10000.0f32..=10000.0)
        // prop::array::uniform16(any::<f32>())
    }

    fn arb_input_feed() -> impl Strategy<Value = Vec<[f32; N]>> {
        prop::collection::vec(arb_frame(), 1..=32)
    }

    proptest! {
        #[test]
        fn prop_cumulative_rms(in_feed in arb_input_feed()) {
            let mut calc = CumulativeRms::<[f32; N], N>::default();

            // NOTE: Older less-numerically stable version for comparison.
            // let expected: [f32; N] = {
            //     let mut exp: [f32; N] = Frame::EQUILIBRIUM;

            //     for frame in in_feed.iter().copied() {
            //         exp.zip_transform(frame, |e, x| e + x * x);
            //     }

            //     let len_f = in_feed.len() as f32;

            //     exp.apply(|x| Float::sqrt(x / len_f))
            // };

            let expected: [f32; N] = {
                let mut exp: [f32; N] = Frame::EQUILIBRIUM;

                for (frame, count) in in_feed.iter().copied().zip(1..) {
                    exp.zip_transform(frame, |e, x| {
                        let x = x * x;
                        e + (x - e) / count as f32
                    });
                }

                exp.apply(Float::sqrt)
            };

            for frame in in_feed {
                calc.advance(frame);
            }

            let produced = calc.current();

            assert_relative_eq!(produced.as_slice(), expected.as_slice());
        }
    }
}
