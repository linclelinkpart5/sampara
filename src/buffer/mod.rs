pub mod fixed;

pub use fixed::Fixed;

pub trait Storage: AsRef<[Self::Item]> + AsMut<[Self::Item]> {
    type Item: Copy + PartialEq;
}

// Would love to be able to do this, but `I` is unconstrained.
// impl<I, A> Storage for A
// where
//     I: Copy + PartialEq,
//     A: AsRef<[Self::Item]> + AsMut<[Self::Item]>,
// {
//     type Item = I;
// }

impl<'a, I> Storage for &'a mut [I]
where
    I: Copy + PartialEq,
{
    type Item = I;
}

impl<I, const N: usize> Storage for [I; N]
where
    I: Copy + PartialEq,
{
    type Item = I;
}

impl<'a, I, const N: usize> Storage for &'a mut [I; N]
where
    I: Copy + PartialEq,
{
    type Item = I;
}

impl<I> Storage for Box<[I]>
where
    I: Copy + PartialEq,
{
    type Item = I;
}

impl<I> Storage for Vec<I>
where
    I: Copy + PartialEq,
{
    type Item = I;
}
