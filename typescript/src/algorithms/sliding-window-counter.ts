import { RateLimiter, RateLimiterStats } from './rate-limiter.interface';

interface WindowData {
  prevCount: number;
  currCount: number;
  prevWindowStart: number;
  currWindowStart: number;
}

export class SlidingWindowCounter implements RateLimiter {
  private windows: Map<string, WindowData> = new Map();
  private limit: number;
  private windowSize: number;

  constructor(limit: number, windowSizeMs: number) {
    this.limit = limit;
    this.windowSize = windowSizeMs;
  }

  async allow(key: string): Promise<boolean> {
    const now = Date.now();
    const currentWindowStart = Math.floor(now / this.windowSize) * this.windowSize;
    
    let data = this.windows.get(key);
    
    if (!data) {
      data = {
        prevCount: 0,
        currCount: 0,
        prevWindowStart: currentWindowStart - this.windowSize,
        currWindowStart: currentWindowStart
      };
      this.windows.set(key, data);
    }

    if (data.currWindowStart < currentWindowStart) {
      data.prevCount = data.currCount;
      data.prevWindowStart = data.currWindowStart;
      data.currCount = 0;
      data.currWindowStart = currentWindowStart;
    }

    const windowProgress = (now - currentWindowStart) / this.windowSize;
    const prevWindowWeight = 1 - windowProgress;
    const estimatedCount = data.prevCount * prevWindowWeight + data.currCount;

    if (estimatedCount < this.limit) {
      data.currCount++;
      return true;
    }

    return false;
  }

  async reset(key: string): Promise<void> {
    this.windows.delete(key);
  }

  async getStats(key: string): Promise<RateLimiterStats> {
    const now = Date.now();
    const currentWindowStart = Math.floor(now / this.windowSize) * this.windowSize;
    const data = this.windows.get(key);
    
    if (!data || data.currWindowStart < currentWindowStart) {
      return {
        allowed: true,
        limit: this.limit,
        remaining: this.limit,
        resetAt: new Date(currentWindowStart + this.windowSize)
      };
    }

    const windowProgress = (now - currentWindowStart) / this.windowSize;
    const prevWindowWeight = 1 - windowProgress;
    const estimatedCount = data.prevCount * prevWindowWeight + data.currCount;
    const remaining = Math.max(0, Math.floor(this.limit - estimatedCount));

    return {
      allowed: estimatedCount < this.limit,
      limit: this.limit,
      remaining,
      resetAt: new Date(currentWindowStart + this.windowSize)
    };
  }
}