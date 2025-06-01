use super::{RateLimiter, RateLimiterStats};
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;

#[derive(Clone)]
struct WindowCounter {
    previous_count: usize,
    current_count: usize,
    current_window_start: DateTime<Utc>,
}

pub struct SlidingWindowCounter {
    counters: RwLock<HashMap<String, WindowCounter>>,
    limit: usize,
    window_size_ms: i64,
}

impl SlidingWindowCounter {
    pub fn new(limit: usize, window_size_ms: i64) -> Self {
        Self {
            counters: RwLock::new(HashMap::new()),
            limit,
            window_size_ms,
        }
    }

    fn get_window_start(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        let epoch_ms = now.timestamp_millis();
        let window_start_ms = (epoch_ms / self.window_size_ms) * self.window_size_ms;
        DateTime::from_timestamp_millis(window_start_ms).unwrap()
    }

    fn calculate_weighted_count(&self, counter: &WindowCounter, now: DateTime<Utc>) -> f64 {
        let current_window_start = self.get_window_start(now);
        
        // If we're in a new window, previous window becomes what was current
        if counter.current_window_start < current_window_start {
            let time_in_current = now.signed_duration_since(current_window_start).num_milliseconds() as f64;
            let window_progress = time_in_current / self.window_size_ms as f64;
            
            // Weighted count = previous window count * (1 - progress) + current window count * progress
            counter.current_count as f64 * (1.0 - window_progress)
        } else {
            // We're still in the same window
            let time_in_current = now.signed_duration_since(counter.current_window_start).num_milliseconds() as f64;
            let window_progress = time_in_current / self.window_size_ms as f64;
            
            // Weighted count = previous window count * (1 - progress) + current window count
            counter.previous_count as f64 * (1.0 - window_progress) + counter.current_count as f64
        }
    }
}

impl RateLimiter for SlidingWindowCounter {
    fn allow(&self, key: &str) -> bool {
        let mut counters = self.counters.write();
        let now = Utc::now();
        let current_window_start = self.get_window_start(now);
        
        let counter = counters.entry(key.to_string()).or_insert(WindowCounter {
            previous_count: 0,
            current_count: 0,
            current_window_start,
        });

        // Check if we need to slide the window
        if counter.current_window_start < current_window_start {
            let previous_window_start = current_window_start - Duration::milliseconds(self.window_size_ms);
            
            if counter.current_window_start == previous_window_start {
                // Slide window: current becomes previous
                counter.previous_count = counter.current_count;
            } else {
                // We've skipped windows, reset previous
                counter.previous_count = 0;
            }
            
            counter.current_count = 0;
            counter.current_window_start = current_window_start;
        }

        // Calculate weighted count
        let weighted_count = self.calculate_weighted_count(counter, now);
        
        // Check if we can increment
        if weighted_count + 1.0 <= self.limit as f64 {
            counter.current_count += 1;
            true
        } else {
            false
        }
    }

    fn reset(&self, key: &str) {
        let mut counters = self.counters.write();
        counters.remove(key);
    }

    fn get_stats(&self, key: &str) -> RateLimiterStats {
        let counters = self.counters.read();
        let now = Utc::now();
        let current_window_start = self.get_window_start(now);
        let window_end = current_window_start + Duration::milliseconds(self.window_size_ms);
        
        match counters.get(key) {
            None => RateLimiterStats {
                allowed: true,
                limit: self.limit,
                remaining: self.limit,
                reset_at: window_end,
            },
            Some(counter) => {
                let mut temp_counter = counter.clone();
                
                // Update counter state if needed
                if temp_counter.current_window_start < current_window_start {
                    let previous_window_start = current_window_start - Duration::milliseconds(self.window_size_ms);
                    
                    if temp_counter.current_window_start == previous_window_start {
                        temp_counter.previous_count = temp_counter.current_count;
                    } else {
                        temp_counter.previous_count = 0;
                    }
                    
                    temp_counter.current_count = 0;
                    temp_counter.current_window_start = current_window_start;
                }
                
                let weighted_count = self.calculate_weighted_count(&temp_counter, now);
                let remaining = (self.limit as f64 - weighted_count).max(0.0) as usize;

                RateLimiterStats {
                    allowed: weighted_count < self.limit as f64,
                    limit: self.limit,
                    remaining,
                    reset_at: window_end,
                }
            }
        }
    }
}