pub mod frame;
pub mod sample;

pub use frame::{Frame, Mono, Stereo};
pub use sample::{Sample, ConvertFrom, ConvertInto, Duplex};
