export interface RateLimiter {
  allow(key: string): Promise<boolean>;
  reset(key: string): Promise<void>;
  getStats(key: string): Promise<RateLimiterStats>;
}

export interface RateLimiterStats {
  allowed: boolean;
  limit: number;
  remaining: number;
  resetAt: Date;
}