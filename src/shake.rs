use std::{collections::VecDeque, time::{Duration, Instant}};

#[derive(Default)]
pub struct DragShake {
    samples: VecDeque<(Instant, f32)>,
    fired: bool,
}

impl DragShake {
    pub fn update(&mut self, x: Option<f32>, dragging_file: bool, now: Instant) -> bool {
        if !dragging_file {
            self.samples.clear();
            self.fired = false;
            return false;
        }
        if self.fired { return false; }
        let Some(x) = x else { return false };
        if self.samples.back().is_some_and(|(_, old_x)| (x - old_x).abs() < 3.0) {
            return false;
        }
        self.samples.push_back((now, x));
        while self.samples.front().is_some_and(|(time, _)|
            now.saturating_duration_since(*time) > Duration::from_millis(700)) {
            self.samples.pop_front();
        }
        let Some((_, first)) = self.samples.front() else { return false };
        let mut extreme = *first;
        let mut direction = 0;
        let mut reversals = 0;
        let mut min_x = *first;
        let mut max_x = *first;
        for (_, sample_x) in self.samples.iter().skip(1) {
            min_x = min_x.min(*sample_x);
            max_x = max_x.max(*sample_x);
            let delta = *sample_x - extreme;
            if direction == 0 {
                if delta.abs() >= 20.0 {
                    direction = delta.signum() as i8;
                    extreme = *sample_x;
                }
            } else if delta.signum() as i8 == direction {
                extreme = *sample_x;
            } else if delta.abs() >= 20.0 {
                reversals += 1;
                direction = delta.signum() as i8;
                extreme = *sample_x;
            }
        }
        if reversals >= 3 && max_x - min_x <= 180.0 {
            self.fired = true;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fires_only_for_a_fast_file_drag_shake() {
        let start = Instant::now();
        let mut detector = DragShake::default();
        for (step, x) in [100.0, 135.0, 90.0, 140.0].into_iter().enumerate() {
            assert!(!detector.update(Some(x), true, start + Duration::from_millis(step as u64 * 90)));
        }
        assert!(detector.update(Some(85.0), true, start + Duration::from_millis(360)));
        assert!(!detector.update(Some(140.0), true, start + Duration::from_millis(450)));
        detector.update(None, false, start + Duration::from_millis(500));
        assert!(!detector.update(Some(100.0), true, start + Duration::from_millis(510)));
    }

    #[test]
    fn ignores_slow_or_small_pointer_motion() {
        let start = Instant::now();
        let mut detector = DragShake::default();
        for (step, x) in [100.0, 106.0, 99.0, 105.0, 100.0].into_iter().enumerate() {
            assert!(!detector.update(Some(x), true, start + Duration::from_millis(step as u64 * 90)));
        }
        for (step, x) in [100.0, 135.0, 90.0, 140.0, 85.0].into_iter().enumerate() {
            assert!(!detector.update(Some(x), true, start + Duration::from_secs(step as u64 + 1)));
        }
    }
}
