use super::{RateLimiter, RateLimiterStats};
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;

#[derive(Clone)]
struct Bucket {
    tokens: f64,
    last_refill: DateTime<Utc>,
}

pub struct TokenBucket {
    buckets: RwLock<HashMap<String, Bucket>>,
    capacity: f64,
    refill_rate_ms: i64,
    refill_amount: f64,
}

impl TokenBucket {
    pub fn new(capacity: usize, refill_rate_ms: i64, refill_amount: f64) -> Self {
        Self {
            buckets: RwLock::new(HashMap::new()),
            capacity: capacity as f64,
            refill_rate_ms,
            refill_amount,
        }
    }
}

impl RateLimiter for TokenBucket {
    fn allow(&self, key: &str) -> bool {
        let mut buckets = self.buckets.write();
        let now = Utc::now();
        
        let bucket = buckets.entry(key.to_string()).or_insert(Bucket {
            tokens: self.capacity,
            last_refill: now,
        });

        let time_passed = now.signed_duration_since(bucket.last_refill);
        let refills = time_passed.num_milliseconds() as f64 / self.refill_rate_ms as f64;
        
        if refills >= 1.0 {
            bucket.tokens = (self.capacity).min(bucket.tokens + (refills * self.refill_amount));
            bucket.last_refill = now;
        }

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
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
                reset_at: now + Duration::milliseconds(self.refill_rate_ms),
            },
            Some(bucket) => {
                let time_passed = now.signed_duration_since(bucket.last_refill);
                let refills = time_passed.num_milliseconds() as f64 / self.refill_rate_ms as f64;
                let current_tokens = (self.capacity).min(bucket.tokens + (refills * self.refill_amount));
                
                let next_refill_time = bucket.last_refill + 
                    Duration::milliseconds(((refills + 1.0) * self.refill_rate_ms as f64) as i64);

                RateLimiterStats {
                    allowed: current_tokens >= 1.0,
                    limit: self.capacity as usize,
                    remaining: current_tokens as usize,
                    reset_at: next_refill_time,
                }
            }
        }
    }
}