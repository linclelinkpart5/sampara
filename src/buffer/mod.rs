pub mod fixed;

pub use fixed::Fixed;

pub trait Storage<I>: AsRef<[I]> + AsMut<[I]>
where
    I: Copy + PartialEq,
{}

impl<I, A> Storage<I> for A
where
    I: Copy + PartialEq,
    A: AsRef<[I]> + AsMut<[I]>,
{}
