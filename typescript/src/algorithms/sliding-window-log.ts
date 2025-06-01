import { RateLimiter, RateLimiterStats } from './rate-limiter.interface';

export class SlidingWindowLog implements RateLimiter {
  private logs: Map<string, number[]> = new Map();
  private limit: number;
  private windowSize: number;

  constructor(limit: number, windowSizeMs: number) {
    this.limit = limit;
    this.windowSize = windowSizeMs;
  }

  async allow(key: string): Promise<boolean> {
    const now = Date.now();
    let timestamps = this.logs.get(key) || [];
    
    timestamps = timestamps.filter(ts => ts > now - this.windowSize);
    
    if (timestamps.length < this.limit) {
      timestamps.push(now);
      this.logs.set(key, timestamps);
      return true;
    }

    this.logs.set(key, timestamps);
    return false;
  }

  async reset(key: string): Promise<void> {
    this.logs.delete(key);
  }

  async getStats(key: string): Promise<RateLimiterStats> {
    const now = Date.now();
    const timestamps = this.logs.get(key) || [];
    const validTimestamps = timestamps.filter(ts => ts > now - this.windowSize);
    
    const remaining = Math.max(0, this.limit - validTimestamps.length);
    
    let resetAt: Date;
    if (validTimestamps.length === 0) {
      resetAt = new Date(now + this.windowSize);
    } else {
      const oldestTimestamp = Math.min(...validTimestamps);
      resetAt = new Date(oldestTimestamp + this.windowSize);
    }

    return {
      allowed: validTimestamps.length < this.limit,
      limit: this.limit,
      remaining,
      resetAt
    };
  }
}