use super::{RateLimiter, RateLimiterStats};
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};

pub struct SlidingWindowLog {
    logs: RwLock<HashMap<String, VecDeque<DateTime<Utc>>>>,
    limit: usize,
    window_size_ms: i64,
}

impl SlidingWindowLog {
    pub fn new(limit: usize, window_size_ms: i64) -> Self {
        Self {
            logs: RwLock::new(HashMap::new()),
            limit,
            window_size_ms,
        }
    }

    fn cleanup_old_entries(&self, log: &mut VecDeque<DateTime<Utc>>, now: DateTime<Utc>) {
        let window_start = now - Duration::milliseconds(self.window_size_ms);
        
        // Remove all entries older than the window
        while let Some(&front) = log.front() {
            if front < window_start {
                log.pop_front();
            } else {
                break;
            }
        }
    }
}

impl RateLimiter for SlidingWindowLog {
    fn allow(&self, key: &str) -> bool {
        let mut logs = self.logs.write();
        let now = Utc::now();
        
        let log = logs.entry(key.to_string()).or_insert_with(VecDeque::new);
        
        // Clean up old entries
        self.cleanup_old_entries(log, now);
        
        // Check if we can add a new entry
        if log.len() < self.limit {
            log.push_back(now);
            true
        } else {
            false
        }
    }

    fn reset(&self, key: &str) {
        let mut logs = self.logs.write();
        logs.remove(key);
    }

    fn get_stats(&self, key: &str) -> RateLimiterStats {
        let mut logs = self.logs.write();
        let now = Utc::now();
        
        match logs.get_mut(key) {
            None => RateLimiterStats {
                allowed: true,
                limit: self.limit,
                remaining: self.limit,
                reset_at: now + Duration::milliseconds(self.window_size_ms),
            },
            Some(log) => {
                // Clean up old entries
                self.cleanup_old_entries(log, now);
                
                let current_count = log.len();
                let remaining = self.limit.saturating_sub(current_count);
                
                // Calculate when the oldest entry will expire
                let reset_at = if let Some(&oldest) = log.front() {
                    oldest + Duration::milliseconds(self.window_size_ms)
                } else {
                    now + Duration::milliseconds(self.window_size_ms)
                };

                RateLimiterStats {
                    allowed: current_count < self.limit,
                    limit: self.limit,
                    remaining,
                    reset_at,
                }
            }
        }
    }
}