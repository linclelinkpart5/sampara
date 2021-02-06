#![feature(array_value_iter)]

#[cfg(feature = "biquad")]
pub mod biquad;
pub mod frame;
pub mod sample;
pub mod signal;

pub use frame::{Frame, Mono, Stereo};
pub use sample::{Sample, ConvertFrom, ConvertInto, Duplex};
pub use signal::Signal;
