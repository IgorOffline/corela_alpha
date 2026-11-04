use std::time::{Duration, Instant};

const ACTION_DELAY: Duration = Duration::from_millis(240);

pub struct PlaybackTimer {
    next_action_time: Instant,
}

impl PlaybackTimer {
    #[must_use]
    pub fn new(now: Instant) -> Self {
        Self {
            next_action_time: now + ACTION_DELAY,
        }
    }

    pub fn poll(&mut self, now: Instant) -> bool {
        if now < self.next_action_time {
            return false;
        }
        self.next_action_time += ACTION_DELAY;
        if self.next_action_time <= now {
            self.next_action_time = now + ACTION_DELAY;
        }
        true
    }
}
