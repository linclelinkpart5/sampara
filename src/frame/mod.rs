/// A trait for working generically across blocks of one or more [`Sample`]s,
/// representing sampling values across one or more channels at a single point
/// in time. Each of these blocks is called a "frame".
pub trait Frame<const N: usize>: Copy + Clone + PartialEq {}
