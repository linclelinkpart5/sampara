//! Pure functions and traits for converting between sample types.
//!
//! Each conversion function is performance focused, memory-sensitive, and
//! expects that the user has pre-validated their input.
//!
//! No conversion function will ever cast to a type with a size in bytes larger
//! than the largest between the source and target sample types.
//!
//! The conversion functions do *not* check the range of incoming values for
//! floating point types.
//!
//! Note that floating point samples span the range [-1.0, 1.0). This means that
//! `1.0f32.convert_into::<i16>()` will overflow, as an example.

use crate::Sample;

/// [`Sample`] types that can be converted from another [`Sample`] type.
pub trait FromSample<S>
where
    S: Sample,
{
    /// Convert [`Self`] from another [`Sample`] type. This is analogous
    /// to [`std::convert::From`], but is intended for preserving the same
    /// represented amplitude between sample types.
    ///
    /// ```
    /// use sampara::{Sample, FromSample};
    ///
    /// fn main() {
    ///     let s: i8 = FromSample::from_sample(0.0f32);
    ///     assert_eq!(s, 0);
    ///
    ///     let s: u8 = FromSample::from_sample(0.0f32);
    ///     assert_eq!(s, 128);
    ///
    ///     let s: f32 = FromSample::from_sample(255u8);
    ///     assert_eq!(s, 0.9921875);
    /// }
    /// ```
    fn from_sample(s: S) -> Self;
}

trait Signed: Sample {
    type Signed: Sample;
}

trait Unsigned: Sample {
    type Unsigned: Sample;
}

macro_rules! define_signed_pairs {
    ($(($Unsigned:ty, $Signed:ty)),* $(,)?) => {
        $(
            impl Signed for $Unsigned {
                type Signed = $Signed;
            }

            impl Unsigned for $Signed {
                type Unsigned = $Unsigned;
            }
        )*
    };
}

define_signed_pairs!((u8, i8), (u16, i16), (u32, i32), (u64, i64), (u128, i128),);

macro_rules! conv_x_to_x {
    ($Source:ty => $Target:ty) => {
        impl FromSample<$Source> for $Target {
            #[inline]
            fn from_sample(s: $Source) -> Self {
                const N: u32 = <$Source>::BITS;
                const M: u32 = <$Target>::BITS;

                if N <= M {
                    (s as $Target) << (M - N)
                } else {
                    (s >> (N - M)) as $Target
                }
            }
        }
    };
}

macro_rules! conv_i_to_i {
    ($I_N:ty => $I_M:ty) => {
        conv_x_to_x!($I_N => $I_M);
    };
}

macro_rules! conv_u_to_u {
    ($U_N:ty => $U_M:ty) => {
        conv_x_to_x!($U_N => $U_M);
    };
}

macro_rules! conv_i_to_u {
    ($I_N:ty => $U_M:ty) => {
        impl FromSample<$I_N> for $U_M {
            #[inline]
            fn from_sample(s: $I_N) -> Self {
                const N: u32 = <$I_N>::BITS;
                const M: u32 = <$U_M>::BITS;

                if N > M {
                    <$U_M>::from_sample(<$U_M as Signed>::Signed::from_sample(s))
                } else {
                    if s < 0 {
                        ((s + <$I_N>::MAX + 1) as $U_M) << (M - N)
                    } else {
                        ((s as $U_M) + (<$I_N>::MAX as $U_M) + 1) << (M - N)
                    }
                }
            }
        }
    };
}

macro_rules! conv_u_to_i {
    ($U_N:ty => $I_M:ty) => {
        impl FromSample<$U_N> for $I_M {
            #[inline]
            fn from_sample(s: $U_N) -> Self {
                const N: u32 = <$U_N>::BITS;
                const M: u32 = <$I_M>::BITS;

                const ORIGIN: $U_N = (1 as $U_N).rotate_right(1);

                if N > M {
                    <$I_M>::from_sample(<$I_M as Unsigned>::Unsigned::from_sample(s))
                } else if N == M {
                    if s < ORIGIN {
                        (s as $I_M) - <$I_M>::MAX - 1
                    } else {
                        (s - ORIGIN) as $I_M
                    }
                } else {
                    ((s as $I_M) - (ORIGIN as $I_M)) << (M - N)
                }
            }
        }
    };
}

macro_rules! conv_i_to_f {
    ($I_N:ty => $F_M:ty) => {
        impl FromSample<$I_N> for $F_M {
            #[inline]
            fn from_sample(s: $I_N) -> Self {
                (s as $F_M) / -(<$I_N>::MIN as $F_M)
            }
        }
    };
}

macro_rules! conv_u_to_f {
    ($U_N:ty => $F_M:ty) => {
        impl FromSample<$U_N> for $F_M {
            #[inline]
            fn from_sample(s: $U_N) -> Self {
                <$F_M>::from_sample(<$U_N as Signed>::Signed::from_sample(s))
            }
        }
    };
}

macro_rules! conv_f_to_i {
    ($F_N:ty => $I_M:ty) => {
        impl FromSample<$F_N> for $I_M {
            #[inline]
            fn from_sample(s: $F_N) -> Self {
                (s * -(<$I_M>::MIN as $F_N)) as $I_M
            }
        }
    };
}

macro_rules! conv_f_to_u {
    ($F_N:ty => $U_M:ty) => {
        impl FromSample<$F_N> for $U_M {
            #[inline]
            fn from_sample(s: $F_N) -> Self {
                <$U_M>::from_sample(<$U_M as Signed>::Signed::from_sample(s))
            }
        }
    };
}

macro_rules! conv_f_to_f {
    ($F_N:ty => $F_M:ty) => {
        impl FromSample<$F_N> for $F_M {
            #[inline]
            fn from_sample(s: $F_N) -> Self {
                s as $F_M
            }
        }
    };
}

macro_rules! one_to_many {
    ($conv_macro:ident, $S:ty => [$($Tx:ty),+ $(,)?]) => {
        $(
            $conv_macro!($S => $Tx);
        )+
    };
}

macro_rules! many_to_many {
    ($conv_macro:ident, [$($Sx:ty),+ $(,)?] => [$($Tx:ty),+ $(,)?]) => {
        many_to_many!(@internal $conv_macro, [$($Sx),+] [$($Tx),+]);
    };
    (@internal $conv_macro:ident, [$($Sx:ty),+] $others:tt) => {
        $(
            one_to_many!($conv_macro, $Sx => $others);
        )+
    };
}

// `iX` -> `iY`
many_to_many!(conv_i_to_i, [i8, i16, i32, i64, i128] => [i8, i16, i32, i64, i128]);

// `iX` -> `uY`
many_to_many!(conv_i_to_u, [i8, i16, i32, i64, i128] => [u8, u16, u32, u64, u128]);

// `uX` -> `iY`
many_to_many!(conv_u_to_i, [u8, u16, u32, u64, u128] => [i8, i16, i32, i64, i128]);

// `uX` -> `uY`
many_to_many!(conv_u_to_u, [u8, u16, u32, u64, u128] => [u8, u16, u32, u64, u128]);

// `fX` -> `fY`
many_to_many!(conv_f_to_f, [f32, f64] => [f32, f64]);

// `iX` -> `fY`
many_to_many!(conv_i_to_f, [i8, i16, i32, i64, i128] => [f32, f64]);

// `uX` -> `fY`
many_to_many!(conv_u_to_f, [u8, u16, u32, u64, u128] => [f32, f64]);

// `fX` -> `iY`
many_to_many!(conv_f_to_i, [f32, f64] => [i8, i16, i32, i64, i128]);

// `fX` -> `uY`
many_to_many!(conv_f_to_u, [f32, f64] => [u8, u16, u32, u64, u128]);

pub trait IntoSample<S>
where
    S: Sample,
{
    /// Convert [`Self`] into another [`Sample`] type. This is analogous
    /// to [`std::convert::Into`], but is intended for preserving the same
    /// represented amplitude between sample types.
    ///
    /// This trait has a blanket implementation for all types that implement
    /// [`FromSample`].
    ///
    /// ```
    /// use sampara::{Sample, IntoSample};
    ///
    /// fn main() {
    ///     let s: i8 = 0.0f32.into_sample();
    ///     assert_eq!(s, 0);
    ///
    ///     let s: u8 = 0.0f32.into_sample();
    ///     assert_eq!(s, 128);
    ///
    ///     let s: f32 = 255u8.into_sample();
    ///     assert_eq!(s, 0.9921875);
    /// }
    /// ```
    fn into_sample(self) -> S;
}

impl<T, U> IntoSample<U> for T
where
    T: Sample,
    U: FromSample<T> + Sample,
{
    fn into_sample(self) -> U {
        U::from_sample(self)
    }
}

#[cfg(test)]
mod tests {}
