#![feature(array_map, array_methods, array_zip, iter_advance_by, option_result_unwrap_unchecked)]

pub mod biquad;
pub mod buffer;
pub mod components;
pub mod frame;
pub mod generator;
pub mod interpolate;
pub mod sample;
pub mod signal;
pub mod stats;
pub mod wavegen;
pub mod window;

pub use components::*;
pub use frame::{Frame, Mono, Stereo};
pub use sample::{Sample, ConvertFrom, ConvertInto, Duplex};
pub use signal::Signal;
