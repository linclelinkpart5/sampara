use crate::sample::FloatSample;

pub trait Interpolant {
    type Inter: FloatSample;

    fn step(&mut self) -> (Self::Inter, usize);
}

pub struct Fixed<X: FloatSample> {
    accum: X,
    delta: X,
}

impl<X: FloatSample> Interpolant for Fixed<X> {
    type Inter = X;

    fn step(&mut self) -> (Self::Inter, usize) {
        let t = self.accum;

        self.accum = self.accum + self.delta;

        let mut frames_to_adv = 0;

        while self.accum >= X::one() {
            self.accum = self.accum - X::one();
            frames_to_adv += 1;
        }

        (t, frames_to_adv)
    }
}

pub struct ResampleRational<X: FloatSample> {
    inter_pts_add: usize,
    after_pts_rem: usize,
    i: usize,
    _marker: std::marker::PhantomData<X>
}

impl<X: FloatSample> ResampleRational<X> {
    pub fn new(to_add: usize, to_rem: usize) -> Self {
        // TODO: Add more robust logic to simplify the resampling ratio.
        let (to_add, to_rem) = if to_add == to_rem {
            (0, 0)
        } else {
            (to_add, to_rem)
        };

        Self {
            inter_pts_add: to_add,
            after_pts_rem: to_rem,
            i: 0,
            _marker: Default::default(),
        }
    }
}

impl<X: FloatSample> Interpolant for ResampleRational<X> {
    type Inter = X;

    fn step(&mut self) -> (Self::Inter, usize) {
        let mut frames_to_adv = 0;

        assert!(self.i <= self.inter_pts_add);

        let x = if self.i == 0 {
            X::zero()
        }
        else {
            X::from(self.i).unwrap() / (X::one() + X::from(self.inter_pts_add).unwrap())
        };

        // NOTE: This is an inclusive end bound, so this runs (N+1) times!
        for _ in 0..=self.after_pts_rem {
            if self.i >= self.inter_pts_add {
                self.i = 0;
                frames_to_adv += 1;
            }
            else {
                self.i += 1;
            }
        }

        (x, frames_to_adv)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_rational() {
        // No resampling, passthrough.
        let mut rr = ResampleRational::<f32>::new(0, 0);

        for _ in 0..=9 {
            assert_eq!(rr.step(), (0.0, 1));
        }

        // Upsample by 2, i.e. add 1 extra sampling point between each existing
        // sample.
        let mut rr = ResampleRational::<f32>::new(1, 0);

        for i in 0..=9 {
            if i % 2 == 0 {
                assert_eq!(rr.step(), (0.0, 0));
            }
            else {
                assert_eq!(rr.step(), (0.5, 1));
            }
        }

        // Upsample by 4, i.e. add 3 extra sampling points between each existing
        // sample.
        let mut rr = ResampleRational::<f32>::new(3, 0);

        for i in 0..=9 {
            if i % 4 == 0 {
                assert_eq!(rr.step(), (0.00, 0));
            }
            else if i % 4 == 1 {
                assert_eq!(rr.step(), (0.25, 0));
            }
            else if i % 4 == 2 {
                assert_eq!(rr.step(), (0.50, 0));
            }
            else {
                assert_eq!(rr.step(), (0.75, 1));
            }
        }

        // Downsample by 2, i.e. drop 1 sample between each kept sample.
        let mut rr = ResampleRational::<f32>::new(0, 1);

        for _ in 0..=9 {
            assert_eq!(rr.step(), (0.0, 2));
        }

        // Downsample by 4, i.e. drop 3 samples between each kept sample.
        let mut rr = ResampleRational::<f32>::new(0, 3);

        for _ in 0..=9 {
            assert_eq!(rr.step(), (0.0, 4));
        }
    }
}
