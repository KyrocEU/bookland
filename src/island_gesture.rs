use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Open,
    Close,
    Previous,
    Next,
}

pub struct IslandGesture {
    horizontal: f32,
    vertical: f32,
    last_event: Option<Instant>,
    fired: bool,
}

impl Default for IslandGesture {
    fn default() -> Self {
        Self { horizontal: 0.0, vertical: 0.0, last_event: None, fired: false }
    }
}

impl IslandGesture {
    pub fn update(&mut self, delta: (f32, f32), now: Instant, expanded: bool,
        vertical_enabled: bool, horizontal_enabled: bool, invert_media: bool) -> Option<Action> {
        if self.last_event.is_some_and(|last| now.duration_since(last) > Duration::from_millis(300)) {
            self.horizontal = 0.0;
            self.vertical = 0.0;
            self.fired = false;
        }
        if delta.0 == 0.0 && delta.1 == 0.0 { return None; }
        self.last_event = Some(now);
        if self.fired { return None; }
        self.horizontal += delta.0;
        self.vertical += delta.1;
        // Egui scroll coordinates follow content motion: positive is down/right.
        // Require a dominant axis so an angled scroll cannot change the track.
        let horizontal = self.horizontal.abs() > self.vertical.abs() * 1.5;
        let vertical = self.vertical.abs() > self.horizontal.abs() * 1.5;
        let action = if horizontal && horizontal_enabled && self.horizontal.abs() >= 55.0 {
            Some(if (self.horizontal > 0.0) != invert_media { Action::Previous }
                else { Action::Next })
        } else if vertical && vertical_enabled && self.vertical.abs() >= 55.0 {
            match (expanded, self.vertical > 0.0) {
                (false, true) => Some(Action::Open),
                (true, false) => Some(Action::Close),
                _ => None,
            }
        } else { None };
        if action.is_some() { self.fired = true; }
        action
    }

    pub fn reset(&mut self) { *self = Self::default(); }
}

#[cfg(test)]
mod tests {
    use super::{Action, IslandGesture};
    use std::time::{Duration, Instant};

    #[test]
    fn one_action_per_swipe_then_rearms_after_rest() {
        let mut gesture = IslandGesture::default();
        let now = Instant::now();
        assert_eq!(gesture.update((0.0, 30.0), now, false, true, true, false), None);
        assert_eq!(gesture.update((0.0, 30.0), now, false, true, true, false), Some(Action::Open));
        assert_eq!(gesture.update((0.0, -80.0), now, true, true, true, false), None);
        assert_eq!(gesture.update((0.0, -60.0), now + Duration::from_millis(350), true,
            true, true, false), Some(Action::Close));
    }

    #[test]
    fn diagonal_scroll_does_not_skip_media() {
        let mut gesture = IslandGesture::default();
        assert_eq!(gesture.update((70.0, 60.0), Instant::now(), true, true, true, false), None);
    }
}
