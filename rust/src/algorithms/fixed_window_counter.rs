use super::{RateLimiter, RateLimiterStats};
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;

#[derive(Clone)]
struct Window {
    count: usize,
    window_start: DateTime<Utc>,
}

pub struct FixedWindowCounter {
    windows: RwLock<HashMap<String, Window>>,
    limit: usize,
    window_size_ms: i64,
}

impl FixedWindowCounter {
    pub fn new(limit: usize, window_size_ms: i64) -> Self {
        Self {
            windows: RwLock::new(HashMap::new()),
            limit,
            window_size_ms,
        }
    }

    fn get_window_start(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        let epoch_ms = now.timestamp_millis();
        let window_start_ms = (epoch_ms / self.window_size_ms) * self.window_size_ms;
        DateTime::from_timestamp_millis(window_start_ms).unwrap()
    }
}

impl RateLimiter for FixedWindowCounter {
    fn allow(&self, key: &str) -> bool {
        let mut windows = self.windows.write();
        let now = Utc::now();
        let current_window_start = self.get_window_start(now);
        
        let window = windows.entry(key.to_string()).or_insert(Window {
            count: 0,
            window_start: current_window_start,
        });

        // Reset counter if we're in a new window
        if window.window_start < current_window_start {
            window.count = 0;
            window.window_start = current_window_start;
        }

        // Check if we can increment
        if window.count < self.limit {
            window.count += 1;
            true
        } else {
            false
        }
    }

    fn reset(&self, key: &str) {
        let mut windows = self.windows.write();
        windows.remove(key);
    }

    fn get_stats(&self, key: &str) -> RateLimiterStats {
        let windows = self.windows.read();
        let now = Utc::now();
        let current_window_start = self.get_window_start(now);
        let window_end = current_window_start + Duration::milliseconds(self.window_size_ms);
        
        match windows.get(key) {
            None => RateLimiterStats {
                allowed: true,
                limit: self.limit,
                remaining: self.limit,
                reset_at: window_end,
            },
            Some(window) => {
                // Check if window is current
                let count = if window.window_start < current_window_start {
                    0
                } else {
                    window.count
                };

                RateLimiterStats {
                    allowed: count < self.limit,
                    limit: self.limit,
                    remaining: self.limit.saturating_sub(count),
                    reset_at: window_end,
                }
            }
        }
    }
}