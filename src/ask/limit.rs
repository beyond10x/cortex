//! `ask.limits.per_identity_per_minute`: the questions of one identity in a sliding window of 60
//! seconds.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_secs(60);

/// Above this many identities, those with no question in the window are forgotten.
const PRUNE_ABOVE: usize = 10_000;

pub struct RateLimiter {
    per_minute: usize,
    asked: HashMap<String, VecDeque<Instant>>,
}

impl RateLimiter {
    pub fn new(per_minute: usize) -> Self {
        Self {
            per_minute,
            asked: HashMap::new(),
        }
    }

    /// Admits a question of `identity` at `now`, or answers the whole seconds until the oldest
    /// question of its window leaves it, at least 1.
    pub fn admit(&mut self, identity: &str, now: Instant) -> Result<(), i64> {
        if self.asked.len() > PRUNE_ABOVE {
            self.asked.retain(|_, times| {
                times
                    .back()
                    .is_some_and(|t| now.saturating_duration_since(*t) < WINDOW)
            });
        }
        let times = self.asked.entry(identity.to_string()).or_default();
        while times
            .front()
            .is_some_and(|t| now.saturating_duration_since(*t) >= WINDOW)
        {
            times.pop_front();
        }
        if times.len() >= self.per_minute {
            let oldest = *times.front().expect("a full window holds a question");
            let wait = WINDOW.saturating_sub(now.saturating_duration_since(oldest));
            let seconds = wait.as_secs() + u64::from(wait.subsec_nanos() > 0);
            return Err(seconds.max(1) as i64);
        }
        times.push_back(now);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_question_past_the_limit_waits_for_the_oldest_to_leave_the_window() {
        let mut limiter = RateLimiter::new(2);
        let start = Instant::now();
        assert_eq!(limiter.admit("a", start), Ok(()));
        assert_eq!(limiter.admit("a", start + Duration::from_secs(10)), Ok(()));
        assert_eq!(limiter.admit("b", start + Duration::from_secs(10)), Ok(()));
        assert_eq!(
            limiter.admit("a", start + Duration::from_millis(20_500)),
            Err(40)
        );
        assert_eq!(limiter.admit("a", start + Duration::from_secs(60)), Ok(()));
    }
}
