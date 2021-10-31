use crate::sample::FloatSample;

pub trait Interpolant {
    type Inter: FloatSample;

    fn step(&mut self) -> (Self::Inter, usize);
}

pub struct Fixed<X: FloatSample> {
    accum: X,
    delta: X,
}

impl<X: FloatSample> Fixed<X> {
    pub fn new(delta: X) -> Self {
        assert!(delta > X::zero());
        assert!(delta.is_finite());

        Self {
            accum: X::zero(),
            delta,
        }
    }
}

impl<X: FloatSample> Interpolant for Fixed<X> {
    type Inter = X;

    fn step(&mut self) -> (Self::Inter, usize) {
        assert!(self.delta > X::zero());
        assert!(self.accum >= X::zero());
        assert!(self.accum < X::one());

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
        assert!(self.i <= self.inter_pts_add);

        let mut frames_to_adv = 0;

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

    use proptest::prelude::*;

    const MAX_DELTA: f32 = 16.0;
    const MAX_TO_ADD: usize = 16;
    const MAX_TO_REM: usize = MAX_TO_ADD;
    const NUM_STEPS: usize = 1000;

    proptest! {
        #[test]
        fn fixed(inv_delta in 0.0..MAX_DELTA) {
            let delta = MAX_DELTA - inv_delta;
            let mut accum = 0.0;

            let mut fixed = Fixed::new(delta);

            for _ in 0..NUM_STEPS {
                let x = accum;

                let mut adv = 0;
                accum += delta;
                while accum >= 1.0 {
                    accum -= 1.0;
                    adv += 1;
                }

                assert_eq!(fixed.step(), (x, adv));
            }
        }

        #[test]
        fn resample_rational(to_add in 0usize..=MAX_TO_ADD, to_rem in 0usize..=MAX_TO_REM) {
            let mut rr = ResampleRational::<f32>::new(to_add, to_rem);

            for t in (0..NUM_STEPS).into_iter().step_by(to_rem + 1) {
                let i = t % (to_add + 1);

                let x = i as f32 / (to_add + 1) as f32;

                let adv = (i + to_rem + 1) / (to_add + 1);

                assert_eq!(rr.step(), (x, adv));
            }
        }
    }
}
