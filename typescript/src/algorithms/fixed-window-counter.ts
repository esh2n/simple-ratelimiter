import { RateLimiter, RateLimiterStats } from './rate-limiter.interface';

interface Window {
  count: number;
  startTime: number;
}

export class FixedWindowCounter implements RateLimiter {
  private windows: Map<string, Window> = new Map();
  private limit: number;
  private windowSize: number;

  constructor(limit: number, windowSizeMs: number) {
    this.limit = limit;
    this.windowSize = windowSizeMs;
  }

  async allow(key: string): Promise<boolean> {
    const now = Date.now();
    const currentWindow = Math.floor(now / this.windowSize);
    
    let window = this.windows.get(key);
    
    if (!window || Math.floor(window.startTime / this.windowSize) !== currentWindow) {
      window = {
        count: 0,
        startTime: currentWindow * this.windowSize
      };
      this.windows.set(key, window);
    }

    if (window.count < this.limit) {
      window.count++;
      return true;
    }

    return false;
  }

  async reset(key: string): Promise<void> {
    this.windows.delete(key);
  }

  async getStats(key: string): Promise<RateLimiterStats> {
    const now = Date.now();
    const currentWindow = Math.floor(now / this.windowSize);
    const window = this.windows.get(key);
    
    if (!window || Math.floor(window.startTime / this.windowSize) !== currentWindow) {
      return {
        allowed: true,
        limit: this.limit,
        remaining: this.limit,
        resetAt: new Date((currentWindow + 1) * this.windowSize)
      };
    }

    const remaining = Math.max(0, this.limit - window.count);
    const resetAt = new Date((currentWindow + 1) * this.windowSize);

    return {
      allowed: window.count < this.limit,
      limit: this.limit,
      remaining,
      resetAt
    };
  }
}