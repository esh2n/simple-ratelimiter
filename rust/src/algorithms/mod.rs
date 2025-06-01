pub mod token_bucket;
pub mod leaky_bucket;
pub mod fixed_window_counter;
pub mod sliding_window_log;
pub mod sliding_window_counter;

use chrono::{DateTime, Utc};
use serde::Serialize;

pub trait RateLimiter: Send + Sync {
    fn allow(&self, key: &str) -> bool;
    fn reset(&self, key: &str);
    fn get_stats(&self, key: &str) -> RateLimiterStats;
}

#[derive(Debug, Clone, Serialize)]
pub struct RateLimiterStats {
    pub allowed: bool,
    pub limit: usize,
    pub remaining: usize,
    pub reset_at: DateTime<Utc>,
}