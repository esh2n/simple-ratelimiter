import { RateLimiter, RateLimiterStats } from './rate-limiter.interface';

interface LeakyBucketData {
  volume: number;
  lastLeak: number;
}

export class LeakyBucket implements RateLimiter {
  private buckets: Map<string, LeakyBucketData> = new Map();
  private capacity: number;
  private leakRate: number;

  constructor(capacity: number, leakRatePerSecond: number) {
    this.capacity = capacity;
    this.leakRate = leakRatePerSecond;
  }

  async allow(key: string): Promise<boolean> {
    const now = Date.now();
    let bucket = this.buckets.get(key);

    if (!bucket) {
      bucket = {
        volume: 0,
        lastLeak: now
      };
      this.buckets.set(key, bucket);
    }

    const timePassed = (now - bucket.lastLeak) / 1000;
    const leaked = timePassed * this.leakRate;
    
    bucket.volume = Math.max(0, bucket.volume - leaked);
    bucket.lastLeak = now;

    if (bucket.volume + 1 <= this.capacity) {
      bucket.volume += 1;
      return true;
    }

    return false;
  }

  async reset(key: string): Promise<void> {
    this.buckets.delete(key);
  }

  async getStats(key: string): Promise<RateLimiterStats> {
    const now = Date.now();
    const bucket = this.buckets.get(key);
    
    if (!bucket) {
      return {
        allowed: true,
        limit: this.capacity,
        remaining: this.capacity,
        resetAt: new Date(now + (this.capacity / this.leakRate) * 1000)
      };
    }

    const timePassed = (now - bucket.lastLeak) / 1000;
    const leaked = timePassed * this.leakRate;
    const currentVolume = Math.max(0, bucket.volume - leaked);
    const remaining = Math.max(0, this.capacity - currentVolume);

    const timeToEmpty = currentVolume / this.leakRate;
    
    return {
      allowed: currentVolume + 1 <= this.capacity,
      limit: this.capacity,
      remaining: Math.floor(remaining),
      resetAt: new Date(now + timeToEmpty * 1000)
    };
  }
}