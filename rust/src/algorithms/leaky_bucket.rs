use super::{RateLimiter, RateLimiterStats};
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;

#[derive(Clone)]
struct Bucket {
    water_level: f64,
    last_leak: DateTime<Utc>,
}

pub struct LeakyBucket {
    buckets: RwLock<HashMap<String, Bucket>>,
    capacity: f64,
    leak_rate_ms: f64, // amount leaked per millisecond
}

impl LeakyBucket {
    pub fn new(capacity: usize, leak_rate_per_second: f64) -> Self {
        Self {
            buckets: RwLock::new(HashMap::new()),
            capacity: capacity as f64,
            leak_rate_ms: leak_rate_per_second / 1000.0,
        }
    }
}

impl RateLimiter for LeakyBucket {
    fn allow(&self, key: &str) -> bool {
        let mut buckets = self.buckets.write();
        let now = Utc::now();
        
        let bucket = buckets.entry(key.to_string()).or_insert(Bucket {
            water_level: 0.0,
            last_leak: now,
        });

        // Calculate water leaked since last update
        let time_passed = now.signed_duration_since(bucket.last_leak);
        let leaked = time_passed.num_milliseconds() as f64 * self.leak_rate_ms;
        
        // Update water level (can't go below 0)
        bucket.water_level = (bucket.water_level - leaked).max(0.0);
        bucket.last_leak = now;

        // Check if we can add 1 unit of water
        if bucket.water_level + 1.0 <= self.capacity {
            bucket.water_level += 1.0;
            true
        } else {
            false
        }
    }

    fn reset(&self, key: &str) {
        let mut buckets = self.buckets.write();
        buckets.remove(key);
    }

    fn get_stats(&self, key: &str) -> RateLimiterStats {
        let buckets = self.buckets.read();
        let now = Utc::now();
        
        match buckets.get(key) {
            None => RateLimiterStats {
                allowed: true,
                limit: self.capacity as usize,
                remaining: self.capacity as usize,
                reset_at: now,
            },
            Some(bucket) => {
                // Calculate current water level
                let time_passed = now.signed_duration_since(bucket.last_leak);
                let leaked = time_passed.num_milliseconds() as f64 * self.leak_rate_ms;
                let current_level = (bucket.water_level - leaked).max(0.0);
                
                // Calculate time until bucket is empty
                let time_to_empty = if current_level > 0.0 {
                    Duration::milliseconds((current_level / self.leak_rate_ms) as i64)
                } else {
                    Duration::milliseconds(0)
                };

                RateLimiterStats {
                    allowed: current_level + 1.0 <= self.capacity,
                    limit: self.capacity as usize,
                    remaining: (self.capacity - current_level) as usize,
                    reset_at: now + time_to_empty,
                }
            }
        }
    }
}