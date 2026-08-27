use dashmap::DashMap;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateKind {
    Speak,
    Clone,
}

pub struct RateLimiter {
    speak_max: u32,
    clone_max: u32,
    speak_windows: DashMap<i64, VecDeque<Instant>>,
    clone_windows: DashMap<i64, VecDeque<Instant>>,
}

impl RateLimiter {
    pub fn new(speak_per_min: u32, clone_per_hr: u32) -> Self {
        Self {
            speak_max: speak_per_min.max(1),
            clone_max: clone_per_hr.max(1),
            speak_windows: DashMap::new(),
            clone_windows: DashMap::new(),
        }
    }

    /// Returns true if the action is allowed (and is recorded).
    pub fn check(&self, user_id: i64, kind: RateKind) -> bool {
        match kind {
            RateKind::Speak => self.check_window(
                user_id,
                &self.speak_windows,
                self.speak_max,
                Duration::from_secs(60),
            ),
            RateKind::Clone => self.check_window(
                user_id,
                &self.clone_windows,
                self.clone_max,
                Duration::from_secs(3600),
            ),
        }
    }

    fn check_window(
        &self,
        user_id: i64,
        map: &DashMap<i64, VecDeque<Instant>>,
        max: u32,
        window: Duration,
    ) -> bool {
        let now = Instant::now();
        let mut entry = map.entry(user_id).or_default();
        let deque = entry.value_mut();
        while deque
            .front()
            .map(|t| now.duration_since(*t) > window)
            .unwrap_or(false)
        {
            deque.pop_front();
        }
        if deque.len() as u32 >= max {
            return false;
        }
        deque.push_back(now);
        true
    }
}
