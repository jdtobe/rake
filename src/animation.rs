//! Slide animation maths. Pure: callers pass in the current time, so nothing here touches a clock.

use std::time::{Duration, Instant};

/// Fast start, gentle stop. Input and output are both in `0.0..=1.0`; input is clamped.
pub fn ease_out_cubic(t: f64) -> f64 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

/// A vertical move of the window from one y coordinate to another.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slide {
    start: Instant,
    duration: Duration,
    from: i32,
    to: i32,
}

impl Slide {
    pub fn new(start: Instant, duration: Duration, from: i32, to: i32) -> Self {
        Slide {
            start,
            duration,
            from,
            to,
        }
    }

    /// A slide from wherever the window is now to `target`. `full_duration` is the time for a
    /// move of the whole `travel` distance, and shorter moves take proportionally less, so
    /// reversing halfway does not feel sluggish.
    pub fn toward(
        now: Instant,
        current: i32,
        target: i32,
        travel: u32,
        full_duration: Duration,
    ) -> Self {
        let duration = if travel == 0 {
            Duration::ZERO
        } else {
            let fraction = f64::from(current.abs_diff(target)) / f64::from(travel);
            full_duration.mul_f64(fraction.min(1.0))
        };
        Slide::new(now, duration, current, target)
    }

    pub fn target(&self) -> i32 {
        self.to
    }

    fn progress(&self, now: Instant) -> f64 {
        if self.duration.is_zero() {
            return 1.0;
        }
        let elapsed = now.saturating_duration_since(self.start);
        (elapsed.as_secs_f64() / self.duration.as_secs_f64()).min(1.0)
    }

    pub fn y_at(&self, now: Instant) -> i32 {
        let distance = f64::from(self.to) - f64::from(self.from);
        self.from + (distance * ease_out_cubic(self.progress(now))).round() as i32
    }

    pub fn is_done(&self, now: Instant) -> bool {
        self.progress(now) >= 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn easing_hits_the_endpoints() {
        assert_eq!(ease_out_cubic(0.0), 0.0);
        assert_eq!(ease_out_cubic(1.0), 1.0);
    }

    #[test]
    fn easing_clamps_out_of_range_input() {
        assert_eq!(ease_out_cubic(-3.0), 0.0);
        assert_eq!(ease_out_cubic(7.0), 1.0);
    }

    #[test]
    fn easing_only_increases_and_starts_fast() {
        let samples: Vec<f64> = (0..=100)
            .map(|i| ease_out_cubic(f64::from(i) / 100.0))
            .collect();
        assert!(samples.windows(2).all(|w| w[0] <= w[1]));
        // Ease-out: more than halfway there after half the time.
        assert!(ease_out_cubic(0.5) > 0.5);
    }

    #[test]
    fn slide_starts_at_from_and_ends_at_to() {
        let t0 = Instant::now();
        let s = Slide::new(t0, ms(200), -400, 0);
        assert_eq!(s.y_at(t0), -400);
        assert_eq!(s.y_at(t0 + ms(200)), 0);
        assert!(!s.is_done(t0));
        assert!(s.is_done(t0 + ms(200)));
    }

    #[test]
    fn slide_stays_at_target_after_it_ends() {
        let t0 = Instant::now();
        let s = Slide::new(t0, ms(200), -400, 0);
        assert_eq!(s.y_at(t0 + ms(5000)), 0);
        assert!(s.is_done(t0 + ms(5000)));
    }

    #[test]
    fn slide_moves_toward_target_monotonically() {
        let t0 = Instant::now();
        let down = Slide::new(t0, ms(200), -400, 0);
        let ys: Vec<i32> = (0..=20).map(|i| down.y_at(t0 + ms(i * 10))).collect();
        assert!(ys.windows(2).all(|w| w[0] <= w[1]), "{ys:?}");
        let up = Slide::new(t0, ms(200), 0, -400);
        let ys: Vec<i32> = (0..=20).map(|i| up.y_at(t0 + ms(i * 10))).collect();
        assert!(ys.windows(2).all(|w| w[0] >= w[1]), "{ys:?}");
    }

    #[test]
    fn zero_duration_is_done_immediately_at_the_target() {
        let t0 = Instant::now();
        let s = Slide::new(t0, Duration::ZERO, -400, 0);
        assert!(s.is_done(t0));
        assert_eq!(s.y_at(t0), 0);
    }

    #[test]
    fn toward_a_full_move_takes_the_full_duration() {
        let t0 = Instant::now();
        let s = Slide::toward(t0, -400, 0, 400, ms(200));
        assert_eq!(s.y_at(t0), -400);
        assert!(!s.is_done(t0 + ms(199)));
        assert!(s.is_done(t0 + ms(200)));
    }

    #[test]
    fn reversing_mid_slide_starts_exactly_where_the_window_is() {
        let t0 = Instant::now();
        let going_down = Slide::new(t0, ms(200), -400, 0);
        let now = t0 + ms(80);
        let here = going_down.y_at(now);
        let back_up = Slide::toward(now, here, -400, 400, ms(200));
        assert_eq!(back_up.y_at(now), here, "no jump when reversing");
        assert_eq!(back_up.target(), -400);
        assert!(back_up.is_done(now + ms(200)));
    }

    #[test]
    fn a_shorter_move_takes_proportionally_less_time() {
        let t0 = Instant::now();
        let half = Slide::toward(t0, -200, 0, 400, ms(200));
        assert!(!half.is_done(t0 + ms(99)));
        assert!(half.is_done(t0 + ms(100)));
    }

    #[test]
    fn already_at_the_target_is_done_at_once() {
        let t0 = Instant::now();
        let s = Slide::toward(t0, 0, 0, 400, ms(200));
        assert!(s.is_done(t0));
        assert_eq!(s.y_at(t0), 0);
    }

    #[test]
    fn zero_travel_or_zero_duration_does_not_divide_by_zero() {
        let t0 = Instant::now();
        assert!(Slide::toward(t0, 0, 0, 0, ms(200)).is_done(t0));
        assert!(Slide::toward(t0, -400, 0, 400, Duration::ZERO).is_done(t0));
    }
}
