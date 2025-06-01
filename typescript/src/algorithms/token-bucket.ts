import { RateLimiter, RateLimiterStats } from './rate-limiter.interface';

interface Bucket {
  tokens: number;
  lastRefill: number;
}

export class TokenBucket implements RateLimiter {
  private buckets: Map<string, Bucket> = new Map();
  private capacity: number;
  private refillRate: number;
  private refillAmount: number;

  constructor(capacity: number, refillRate: number, refillAmount: number = 1) {
    this.capacity = capacity;
    this.refillRate = refillRate;
    this.refillAmount = refillAmount;
  }

  async allow(key: string): Promise<boolean> {
    const now = Date.now();
    let bucket = this.buckets.get(key);

    if (!bucket) {
      bucket = {
        tokens: this.capacity,
        lastRefill: now
      };
      this.buckets.set(key, bucket);
    }

    const timePassed = now - bucket.lastRefill;
    const refills = Math.floor(timePassed / this.refillRate);
    
    if (refills > 0) {
      bucket.tokens = Math.min(
        this.capacity,
        bucket.tokens + (refills * this.refillAmount)
      );
      bucket.lastRefill = now;
    }

    if (bucket.tokens >= 1) {
      bucket.tokens--;
      return true;
    }

    return false;
  }

  async reset(key: string): Promise<void> {
    this.buckets.delete(key);
  }

  async getStats(key: string): Promise<RateLimiterStats> {
    const bucket = this.buckets.get(key);
    const now = Date.now();
    
    if (!bucket) {
      return {
        allowed: true,
        limit: this.capacity,
        remaining: this.capacity,
        resetAt: new Date(now + this.refillRate)
      };
    }

    const timePassed = now - bucket.lastRefill;
    const refills = Math.floor(timePassed / this.refillRate);
    const currentTokens = Math.min(
      this.capacity,
      bucket.tokens + (refills * this.refillAmount)
    );

    const nextRefillTime = bucket.lastRefill + 
      ((refills + 1) * this.refillRate);

    return {
      allowed: currentTokens >= 1,
      limit: this.capacity,
      remaining: Math.floor(currentTokens),
      resetAt: new Date(nextRefillTime)
    };
  }
}