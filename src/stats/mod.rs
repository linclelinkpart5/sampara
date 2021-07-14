//! Statistical calculators for processing frames.
//!
//! These statistical calculators include, but are not limited to, mean
//! (average), root mean square (RMS), and min/max. Each of these calculators
//! come in different flavors:
//!
//! * Cumulative, where all previously processed frames factor into the
//! calculation.
//! * Moving, where a finite sized window of the most recent `N` frames factor
//! into the calculation.

#![macro_use]

// LEARN: These macros must be defined before defining submodules, annoyingly.
macro_rules! apply_doc_comment {
    ($doc_comment:expr, { $($tt:tt)* }) => {
        #[doc = $doc_comment]
        $($tt)*
    };
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

pub(crate) mod cumulative;
pub(crate) mod moving;

pub use cumulative::*;
pub use moving::*;

use std::cmp::Ordering;

use crate::Sample;

const EMPTY_BUFFER_MSG: &'static str = "buffer cannot be empty";
const ZERO_FRAMES_MSG: &'static str = "no frames processed yet";

const DO_SQRT: bool = true;
const NO_SQRT: bool = false;
const DO_POW2: bool = true;
const NO_POW2: bool = false;
const DO_MAX: bool = true;
const DO_MIN: bool = false;

fn surpasses<S: Sample, const MAX: bool>(candidate: &S, target: &S) -> bool {
    match candidate.partial_cmp(&target) {
        // The new value does not surpass the target extrema.
        None => false,
        Some(Ordering::Less) if MAX => false,
        Some(Ordering::Greater) if !MAX => false,

        _ => true,
    }
}
