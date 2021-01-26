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
//! `1.0f32.convert_into::<i16>()` will overflow.

use crate::Sample;

macro_rules! conversion_fn {
    ($Rep:ty, $s:ident to_i8 { $body:expr }) => {
        #[inline]
        pub fn to_i8($s: $Rep) -> i8 {
            $body
        }
    };

    ($Rep:ty, $s:ident to_i16 { $body:expr }) => {
        #[inline]
        pub fn to_i16($s: $Rep) -> i16 {
            $body
        }
    };

    ($Rep:ty, $s:ident to_i32 { $body:expr }) => {
        #[inline]
        pub fn to_i32($s: $Rep) -> i32 {
            $body
        }
    };

    ($Rep:ty, $s:ident to_i64 { $body:expr }) => {
        #[inline]
        pub fn to_i64($s: $Rep) -> i64 {
            $body
        }
    };

    ($Rep:ty, $s:ident to_u8 { $body:expr }) => {
        #[inline]
        pub fn to_u8($s: $Rep) -> u8 {
            $body
        }
    };

    ($Rep:ty, $s:ident to_u16 { $body:expr }) => {
        #[inline]
        pub fn to_u16($s: $Rep) -> u16 {
            $body
        }
    };

    ($Rep:ty, $s:ident to_u32 { $body:expr }) => {
        #[inline]
        pub fn to_u32($s: $Rep) -> u32 {
            $body
        }
    };

    ($Rep:ty, $s:ident to_u64 { $body:expr }) => {
        #[inline]
        pub fn to_u64($s: $Rep) -> u64 {
            $body
        }
    };

    ($Rep:ty, $s:ident to_f32 { $body:expr }) => {
        #[inline]
        pub fn to_f32($s: $Rep) -> f32 {
            $body
        }
    };

    ($Rep:ty, $s:ident to_f64 { $body:expr }) => {
        #[inline]
        pub fn to_f64($s: $Rep) -> f64 {
            $body
        }
    };
}

macro_rules! conversion_fns {
    ($Rep:ty, $s:ident $fn_name:tt { $body:expr } $($rest:tt)*) => {
        conversion_fn!($Rep, $s $fn_name { $body });
        conversion_fns!($Rep, $($rest)*);
    };
    ($Rep:ty, ) => {};
}

macro_rules! conversions {
    ($T:ident, $mod_name:ident { $($rest:tt)* }) => {
        pub mod $mod_name {
            conversion_fns!($T, $($rest)*);
        }
    };
}

conversions!(i8, i8 {
    s to_i16 { (s as i16) << 8 }
    s to_i32 { (s as i32) << 24 }
    s to_i64 { (s as i64) << 56 }
    s to_u8 {
        if s < 0 {
            // 128i8 overflows, so we must use 127 + 1 instead.
            (s + 127 + 1) as u8
        } else {
            (s as u8) + 128
        }
    }
    s to_u16 {
        if s < 0 {
            ((s + 127 + 1) as u16) << 8
        } else {
            (s as u16 + 128) << 8
        }
    }
    s to_u32 {
        if s < 0 {
            ((s + 127 + 1) as u32) << 24
        } else {
            (s as u32 + 128) << 24
        }
    }
    s to_u64 {
        if s < 0 {
            ((s + 127 + 1) as u64) << 56
        } else {
            (s as u64 + 128) << 56
        }
    }
    s to_f32 {
        s as f32 / 128.0
    }
    s to_f64 {
        s as f64 / 128.0
    }
});

conversions!(i16, i16 {
    s to_i8 { (s >> 8) as i8 }
    s to_i32 { (s as i32) << 16 }
    s to_i64 { (s as i64) << 48 }
    s to_u8 {
        super::i8::to_u8(to_i8(s))
    }
    s to_u16 {
        if s < 0 {
            // 32_768i16 overflows, so we must use + 1 instead.
            (s + 32_767 + 1) as u16
        } else {
            s as u16 + 32_768
        }
    }
    s to_u32 {
        if s < 0 {
            ((s + 32_767 + 1) as u32) << 16
        } else {
            ((s as u32) + 32_768) << 16
        }
    }
    s to_u64 {
        if s < 0 {
            ((s + 32_767 + 1) as u64) << 48
        } else {
            ((s as u64) + 32_768) << 48
        }
    }
    s to_f32 {
        s as f32 / 32_768.0
    }
    s to_f64 {
        s as f64 / 32_768.0
    }
});

conversions!(i32, i32 {
    s to_i8 { (s >> 24) as i8 }
    s to_i16 { (s >> 16) as i16 }
    s to_i64 { (s as i64) << 32 }
    s to_u8 {
        super::i8::to_u8(to_i8(s))
    }
    s to_u16 {
        super::i16::to_u16(to_i16(s))
    }
    s to_u32 {
        if s < 0 {
            (s + 2_147_483_647 + 1) as u32
        } else {
            s as u32 + 2_147_483_648
        }
    }
    s to_u64 {
        if s < 0 {
            ((s + 2_147_483_647 + 1) as u64) << 32
        } else {
            (s as u64) + 2_147_483_648 << 32
        }
    }
    s to_f32 {
        s as f32 / 2_147_483_648.0
    }
    s to_f64 {
        s as f64 / 2_147_483_648.0
    }
});

conversions!(i64, i64 {
    s to_i8 { (s >> 56) as i8 }
    s to_i16 { (s >> 48) as i16 }
    s to_i32 { (s >> 32) as i32 }
    s to_u8 {
        super::i8::to_u8(to_i8(s))
    }
    s to_u16 {
        super::i16::to_u16(to_i16(s))
    }
    s to_u32 {
        super::i32::to_u32(to_i32(s))
    }
    s to_u64 {
        if s < 0 {
            (s + 9_223_372_036_854_775_807 + 1) as u64
        } else {
            s as u64 + 9_223_372_036_854_775_808
        }
    }
    s to_f32 {
        s as f32 / 9_223_372_036_854_775_808.0
    }
    s to_f64 {
        s as f64 / 9_223_372_036_854_775_808.0
    }
});

conversions!(u8, u8 {
    s to_i8 {
        if s < 128 {
            s as i8 - 127 - 1
        } else {
            (s - 128) as i8
        }
    }
    s to_i16 {
        (s as i16 - 128) << 8
    }
    s to_i32 {
        (s as i32 - 128) << 24
    }
    s to_i64 {
        (s as i64 - 128) << 56
    }
    s to_u16 { (s as u16) << 8 }
    s to_u32 { (s as u32) << 24 }
    s to_u64 { (s as u64) << 56 }
    s to_f32 { super::i8::to_f32(to_i8(s)) }
    s to_f64 { super::i8::to_f64(to_i8(s)) }
});

conversions!(u16, u16 {
    s to_i8 { super::u8::to_i8(to_u8(s)) }
    s to_i16 {
        if s < 32_768 {
            s as i16 - 32_767 - 1
        } else {
            (s - 32_768) as i16
        }
    }
    s to_i32 {
        (s as i32 - 32_768) << 16
    }
    s to_i64 {
        (s as i64 - 32_768) << 48
    }
    s to_u8 { (s >> 8) as u8 }
    s to_u32 { (s as u32) << 16 }
    s to_u64 { (s as u64) << 48 }
    s to_f32 { super::i16::to_f32(to_i16(s)) }
    s to_f64 { super::i16::to_f64(to_i16(s)) }
});

conversions!(u32, u32 {
    s to_i8 { super::u8::to_i8(to_u8(s)) }
    s to_i16 { super::u16::to_i16(to_u16(s)) }
    s to_i32 {
        if s < 2_147_483_648 {
            s as i32 - 2_147_483_647 - 1
        } else {
            (s - 2_147_483_648) as i32
        }
    }
    s to_i64 {
        (s as i64 - 2_147_483_648) << 32
    }
    s to_u8 { (s >> 24) as u8 }
    s to_u16 { (s >> 16) as u16 }
    s to_u64 { (s as u64) << 32 }
    s to_f32 { super::i32::to_f32(to_i32(s)) }
    s to_f64 { super::i32::to_f64(to_i32(s)) }
});

conversions!(u64, u64 {
    s to_i8 { super::u8::to_i8(to_u8(s)) }
    s to_i16 { super::u16::to_i16(to_u16(s)) }
    s to_i32 { super::u32::to_i32(to_u32(s)) }
    s to_i64 {
        if s < 9_223_372_036_854_775_808 {
            s as i64 - 9_223_372_036_854_775_807 - 1
        } else {
            (s - 9_223_372_036_854_775_808) as i64
        }
    }
    s to_u8 { (s >> 56) as u8 }
    s to_u16 { (s >> 48) as u16 }
    s to_u32 { (s >> 32) as u32 }
    s to_f32 { super::i64::to_f32(to_i64(s)) }
    s to_f64 { super::i64::to_f64(to_i64(s)) }
});

// The following conversions assume the sample value is in the range
// [-1.0, 1.0), and will overflow otherwise.
conversions!(f32, f32 {
    s to_i8 { (s * 128.0) as i8 }
    s to_i16 { (s * 32_768.0) as i16 }
    s to_i32 { (s * 2_147_483_648.0) as i32 }
    s to_i64 { (s * 9_223_372_036_854_775_808.0) as i64 }
    s to_u8 { super::i8::to_u8(to_i8(s)) }
    s to_u16 { super::i16::to_u16(to_i16(s)) }
    s to_u32 { super::i32::to_u32(to_i32(s)) }
    s to_u64 { super::i64::to_u64(to_i64(s)) }
    s to_f64 { s as f64 }
});

// The following conversions assume the sample value is in the range
// [-1.0, 1.0), and will overflow otherwise.
conversions!(f64, f64 {
    s to_i8 { (s * 128.0) as i8 }
    s to_i16 { (s * 32_768.0) as i16 }
    s to_i32 { (s * 2_147_483_648.0) as i32 }
    s to_i64 { (s * 9_223_372_036_854_775_808.0) as i64 }
    s to_u8 { super::i8::to_u8(to_i8(s)) }
    s to_u16 { super::i16::to_u16(to_i16(s)) }
    s to_u32 { super::i32::to_u32(to_i32(s)) }
    s to_u64 { super::i64::to_u64(to_i64(s)) }
    s to_f32 { s as f32 }
});

/// Allows converting from one [`Sample`] type into another. This is analogous
/// to [`std::convert::From`], but is intended for preserving the same
/// represented amplitude between sample types.
///
/// ```rust
/// use sampara::{Sample, ConvertFrom};
///
/// fn main() {
///     let s: i8 = ConvertFrom::convert_from(0.0f32);
///     assert_eq!(s, 0);
///
///     let s: u8 = ConvertFrom::convert_from(0.0f32);
///     assert_eq!(s, 128);
///
///     let s: f32 = ConvertFrom::convert_from(255u8);
///     assert_eq!(s, 0.9921875);
/// }
pub trait ConvertFrom<S>
where
    S: Sample,
{
    fn convert_from(s: S) -> Self;
}

// All [`Sample`]s can be converted into themselves trivially.
impl<S> ConvertFrom<S> for S
where
    S: Sample,
{
    fn convert_from(s: S) -> Self {
        s
    }
}

macro_rules! impl_convert_from {
    ($T:ty, $fn_name:ident from $({$U:ident: $Umod:ident})*) => {
        $(
            impl ConvertFrom<$U> for $T {
                #[inline]
                fn convert_from(s: $U) -> Self {
                    self::$Umod::$fn_name(s)
                }
            }
        )*
    };
}

impl_convert_from! {i8, to_i8 from
    {i16:i16} {i32:i32} {i64:i64}
    {u8:u8} {u16:u16} {u32:u32} {u64:u64}
    {f32:f32} {f64:f64}
}

impl_convert_from! {i16, to_i16 from
    {i8:i8} {i32:i32} {i64:i64}
    {u8:u8} {u16:u16} {u32:u32} {u64:u64}
    {f32:f32} {f64:f64}
}

impl_convert_from! {i32, to_i32 from
    {i8:i8} {i16:i16} {i64:i64}
    {u8:u8} {u16:u16} {u32:u32} {u64:u64}
    {f32:f32} {f64:f64}
}

impl_convert_from! {i64, to_i64 from
    {i8:i8} {i16:i16} {i32:i32}
    {u8:u8} {u16:u16} {u32:u32} {u64:u64}
    {f32:f32} {f64:f64}
}

impl_convert_from! {u8, to_u8 from
    {i8:i8} {i16:i16} {i32:i32} {i64:i64}
    {u16:u16} {u32:u32} {u64:u64}
    {f32:f32} {f64:f64}
}

impl_convert_from! {u16, to_u16 from
    {i8:i8} {i16:i16} {i32:i32} {i64:i64}
    {u8:u8} {u32:u32} {u64:u64}
    {f32:f32} {f64:f64}
}

impl_convert_from! {u32, to_u32 from
    {i8:i8} {i16:i16} {i32:i32} {i64:i64}
    {u8:u8} {u16:u16} {u64:u64}
    {f32:f32} {f64:f64}
}

impl_convert_from! {u64, to_u64 from
    {i8:i8} {i16:i16} {i32:i32} {i64:i64}
    {u8:u8} {u16:u16} {u32:u32}
    {f32:f32} {f64:f64}
}

impl_convert_from! {f32, to_f32 from
    {i8:i8} {i16:i16} {i32:i32} {i64:i64}
    {u8:u8} {u16:u16} {u32:u32} {u64:u64}
    {f64:f64}
}

impl_convert_from! {f64, to_f64 from
    {i8:i8} {i16:i16} {i32:i32} {i64:i64}
    {u8:u8} {u16:u16} {u32:u32} {u64:u64}
    {f32:f32}
}

/// Allows converting from one [`Sample`] type into another. This is analogous
/// to [`std::convert::Into`], but is intended for preserving the same
/// represented amplitude between sample types.
///
/// This trait has a blanket implementation for all types that implement
/// [`ConvertFrom`].
///
/// ```rust
/// use sampara::{Sample, ConvertInto};
///
/// fn main() {
///     let s: i8 = 0.0f32.convert_into();
///     assert_eq!(s, 0);
///
///     let s: u8 = 0.0f32.convert_into();
///     assert_eq!(s, 128);
///
///     let s: f32 = 255u8.convert_into();
///     assert_eq!(s, 0.9921875);
/// }
pub trait ConvertInto<S>
where
    S: Sample,
{
    fn convert_into(self) -> S;
}

impl<T, U> ConvertInto<U> for T
where
    T: Sample,
    U: ConvertFrom<T> + Sample,
{
    fn convert_into(self) -> U {
        U::convert_from(self)
    }
}

/// [`Sample`]s that can be converted into and from another [`Sample`] type.
pub trait Duplex<S>: ConvertFrom<S> + ConvertInto<S> where S: Sample {}
impl<S, T> Duplex<S> for T where S: Sample, T: ConvertFrom<S> + ConvertInto<S> {}
