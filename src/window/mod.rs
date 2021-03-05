use num_traits::Float;

use crate::buffer::Buffer;

struct Intersperse(usize, usize);

impl Iterator for Intersperse {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 < self.1 {
            let x = self.0 as f64 / self.1 as f64;
            self.0 += 1;
            Some(x)
        }
        else {
            None
        }
    }
}

/// Helper function to create signal windows. Since windows are symmetric about
/// the y-axis, only one "half" needs to be calculated.
fn symfill<B, F>(buffer: &mut B, mut func: F)
where
    B: Buffer,
    F: FnMut(usize) -> B::Item,
{
    let len = buffer.as_ref().len();

    if len == 0 {
        return;
    }

    let mut l = 0usize;
    // This cannot underflow.
    let mut r = len - 1 - l;

    let mut dest = buffer.as_mut();

    while l < r {
        let x = func(l);

        dest[l] = x;
        dest[r] = x;

        l += 1;
        r -= 1;
    }

    // Fill in the middle odd point, if applicable.
    if l == r {
        dest[l] = func(l);
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Kind {
    Rectangular,
    Triangular,
    // Blackman,
    // BlackmanHarris,
    // Hann,
}

impl Kind {
    fn fill_half_buf(self, buffer: B, include_center: bool) -> (B, Option<B::Item>)
    where
        B: Buffer,
        B::Item: Float,
    {
        let mut center = None;
        let mut dest = buffer.as_mut();
        let len = dest.len();
        let len_f = Float::from(len);

        match self {
            Self::Rectangular => {
                dest.fill(F::one());

                if include_center {
                    center = Some(F::one());
                }
            },
            Self::Triangular => {
                for (n, x) in dest.iter_mut().enumerate() {
                    *x = Float::from(n) / len_f;
                }

                if include_center {
                    center = Some(F::one());
                }
            },
        };

        (buffer, center)
    }

    // pub fn fill_buffer<B, F>(self, buffer: &mut B)
    // where
    //     B: Buffer<F>,
    //     F: Float,
    // {
    //     let len = buffer.as_ref().len();

    //     match self {
    //         Self::Rectangular => buffer.as_mut().fill(F::one()),
    //         Self::Triangular => {

    //         },
    //     }
    // }
}

pub struct HalfWindow<B>
where
    B: Buffer,
{
    left_half: B,
    center: Option<B::Item>,
}

impl<B> HalfWindow<B>
where
    B: Buffer,
{
    pub fn len(&self) -> usize {
        self.left_half().as_ref().len() * 2 + if self.center.is_some() as usize
    }

    pub fn from_kind(kind: Kind, buffer: B, include_center: bool) -> Self {
        todo!()
    }
}
