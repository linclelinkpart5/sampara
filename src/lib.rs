#![feature(associated_type_defaults)]

pub mod frame;
pub mod sample;

pub use sample::{FromSample, IntoSample, Sample};

#[cfg(test)]
mod tests {}
