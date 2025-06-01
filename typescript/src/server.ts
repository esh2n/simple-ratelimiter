import express, { Request, Response, NextFunction } from 'express';
import { RateLimiter } from './algorithms/rate-limiter.interface';
import { TokenBucket } from './algorithms/token-bucket';
import { LeakyBucket } from './algorithms/leaky-bucket';
import { FixedWindowCounter } from './algorithms/fixed-window-counter';
import { SlidingWindowLog } from './algorithms/sliding-window-log';
import { SlidingWindowCounter } from './algorithms/sliding-window-counter';

const app = express();
app.use(express.json());

const rateLimiters: Map<string, RateLimiter> = new Map([
  ['token-bucket', new TokenBucket(10, 1000, 1)],
  ['leaky-bucket', new LeakyBucket(10, 10)],
  ['fixed-window', new FixedWindowCounter(10, 60000)],
  ['sliding-window-log', new SlidingWindowLog(10, 60000)],
  ['sliding-window-counter', new SlidingWindowCounter(10, 60000)]
]);

function createRateLimitMiddleware(algorithm: string) {
  return async (req: Request, res: Response, next: NextFunction) => {
    const limiter = rateLimiters.get(algorithm);
    if (!limiter) {
      return res.status(500).json({ error: 'Invalid rate limiter algorithm' });
    }

    const clientId = req.ip || 'unknown';
    const key = `${algorithm}:${clientId}`;
    
    const allowed = await limiter.allow(key);
    const stats = await limiter.getStats(key);
    
    res.setHeader('X-RateLimit-Limit', stats.limit.toString());
    res.setHeader('X-RateLimit-Remaining', stats.remaining.toString());
    res.setHeader('X-RateLimit-Reset', stats.resetAt.toISOString());
    
    if (!allowed) {
      return res.status(429).json({
        error: 'Too Many Requests',
        retryAfter: Math.ceil((stats.resetAt.getTime() - Date.now()) / 1000)
      });
    }
    
    next();
  };
}

app.get('/api/data/:algorithm', createRateLimitMiddleware('token-bucket'), (req: Request, res: Response) => {
  const algorithm = req.params.algorithm;
  if (rateLimiters.has(algorithm)) {
    const middleware = createRateLimitMiddleware(algorithm);
    return middleware(req, res, () => {
      res.json({
        message: 'Success',
        algorithm,
        timestamp: new Date().toISOString()
      });
    });
  }
  
  res.status(404).json({ error: 'Algorithm not found' });
});

app.get('/api/stats/:algorithm', async (req: Request, res: Response) => {
  const algorithm = req.params.algorithm;
  const limiter = rateLimiters.get(algorithm);
  
  if (!limiter) {
    return res.status(404).json({ error: 'Algorithm not found' });
  }
  
  const clientId = req.ip || 'unknown';
  const key = `${algorithm}:${clientId}`;
  const stats = await limiter.getStats(key);
  
  res.json({
    algorithm,
    stats,
    availableAlgorithms: Array.from(rateLimiters.keys())
  });
});

app.get('/api/algorithms', (req: Request, res: Response) => {
  res.json({
    algorithms: Array.from(rateLimiters.keys()),
    endpoints: {
      data: '/api/data/:algorithm',
      stats: '/api/stats/:algorithm'
    }
  });
});

const PORT = process.env.PORT || 3000;
app.listen(PORT, () => {
  console.log(`TypeScript Rate Limiter Server running on port ${PORT}`);
  console.log(`Available algorithms: ${Array.from(rateLimiters.keys()).join(', ')}`);
});