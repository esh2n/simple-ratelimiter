package algorithms

import (
	"sync"
	"time"
)

type bucket struct {
	tokens     float64
	lastRefill time.Time
}

type TokenBucket struct {
	mu           sync.RWMutex
	buckets      map[string]*bucket
	capacity     float64
	refillRate   time.Duration
	refillAmount float64
}

func NewTokenBucket(capacity int, refillRate time.Duration, refillAmount float64) *TokenBucket {
	return &TokenBucket{
		buckets:      make(map[string]*bucket),
		capacity:     float64(capacity),
		refillRate:   refillRate,
		refillAmount: refillAmount,
	}
}

func (tb *TokenBucket) Allow(key string) bool {
	tb.mu.Lock()
	defer tb.mu.Unlock()

	now := time.Now()
	b, exists := tb.buckets[key]
	if !exists {
		b = &bucket{
			tokens:     tb.capacity,
			lastRefill: now,
		}
		tb.buckets[key] = b
	}

	timePassed := now.Sub(b.lastRefill)
	refills := float64(timePassed) / float64(tb.refillRate)
	
	if refills >= 1 {
		b.tokens = min(tb.capacity, b.tokens+(refills*tb.refillAmount))
		b.lastRefill = now
	}

	if b.tokens >= 1 {
		b.tokens--
		return true
	}

	return false
}

func (tb *TokenBucket) Reset(key string) {
	tb.mu.Lock()
	defer tb.mu.Unlock()
	delete(tb.buckets, key)
}

func (tb *TokenBucket) GetStats(key string) RateLimiterStats {
	tb.mu.RLock()
	defer tb.mu.RUnlock()

	now := time.Now()
	b, exists := tb.buckets[key]
	if !exists {
		return RateLimiterStats{
			Allowed:   true,
			Limit:     int(tb.capacity),
			Remaining: int(tb.capacity),
			ResetAt:   now.Add(tb.refillRate),
		}
	}

	timePassed := now.Sub(b.lastRefill)
	refills := float64(timePassed) / float64(tb.refillRate)
	currentTokens := min(tb.capacity, b.tokens+(refills*tb.refillAmount))

	nextRefillTime := b.lastRefill.Add(time.Duration(float64(tb.refillRate) * (refills + 1)))

	return RateLimiterStats{
		Allowed:   currentTokens >= 1,
		Limit:     int(tb.capacity),
		Remaining: int(currentTokens),
		ResetAt:   nextRefillTime,
	}
}

func min(a, b float64) float64 {
	if a < b {
		return a
	}
	return b
}