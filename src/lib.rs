#![feature(associated_type_bounds)]
#![feature(trait_alias)]

#[cfg(feature = "biquad")]
pub mod biquad;
#[cfg(feature = "buffer")]
pub mod buffer;
pub mod frame;
#[cfg(feature = "generator")]
pub mod generator;
#[cfg(feature = "interpolate")]
pub mod interpolate;
#[cfg(feature = "rms")]
pub mod rms;
pub mod sample;
pub mod signal;
#[cfg(feature = "window")]
pub mod window;

pub use frame::{Frame, Mono, Stereo};
pub use sample::{Sample, ConvertFrom, ConvertInto, Duplex};
pub use signal::Signal;
