pub mod fixed;

pub use fixed::Fixed;

pub trait Buffer: AsRef<[Self::Item]> + AsMut<[Self::Item]> {
    type Item: Copy + PartialEq;
}

// Would love to be able to do this, but `I` is unconstrained.
// impl<I, A> Buffer for A
// where
//     I: Copy + PartialEq,
//     A: AsRef<[Self::Item]> + AsMut<[Self::Item]>,
// {
//     type Item = I;
// }

impl<'a, I> Buffer for &'a mut [I]
where
    I: Copy + PartialEq,
{
    type Item = I;
}

impl<I, const N: usize> Buffer for [I; N]
where
    I: Copy + PartialEq,
{
    type Item = I;
}

impl<'a, I, const N: usize> Buffer for &'a mut [I; N]
where
    I: Copy + PartialEq,
{
    type Item = I;
}

impl<I> Buffer for Box<[I]>
where
    I: Copy + PartialEq,
{
    type Item = I;
}

impl<I> Buffer for Vec<I>
where
    I: Copy + PartialEq,
{
    type Item = I;
}
