package algorithms

import (
	"sync"
	"time"
)

type leakyBucketData struct {
	volume   float64
	lastLeak time.Time
}

type LeakyBucket struct {
	mu       sync.RWMutex
	buckets  map[string]*leakyBucketData
	capacity float64
	leakRate float64
}

func NewLeakyBucket(capacity int, leakRatePerSecond float64) *LeakyBucket {
	return &LeakyBucket{
		buckets:  make(map[string]*leakyBucketData),
		capacity: float64(capacity),
		leakRate: leakRatePerSecond,
	}
}

func (lb *LeakyBucket) Allow(key string) bool {
	lb.mu.Lock()
	defer lb.mu.Unlock()

	now := time.Now()
	b, exists := lb.buckets[key]
	if !exists {
		b = &leakyBucketData{
			volume:   0,
			lastLeak: now,
		}
		lb.buckets[key] = b
	}

	timePassed := now.Sub(b.lastLeak).Seconds()
	leaked := timePassed * lb.leakRate

	b.volume = max(0, b.volume-leaked)
	b.lastLeak = now

	if b.volume+1 <= lb.capacity {
		b.volume++
		return true
	}

	return false
}

func (lb *LeakyBucket) Reset(key string) {
	lb.mu.Lock()
	defer lb.mu.Unlock()
	delete(lb.buckets, key)
}

func (lb *LeakyBucket) GetStats(key string) RateLimiterStats {
	lb.mu.RLock()
	defer lb.mu.RUnlock()

	now := time.Now()
	b, exists := lb.buckets[key]
	if !exists {
		return RateLimiterStats{
			Allowed:   true,
			Limit:     int(lb.capacity),
			Remaining: int(lb.capacity),
			ResetAt:   now.Add(time.Duration(lb.capacity/lb.leakRate) * time.Second),
		}
	}

	timePassed := now.Sub(b.lastLeak).Seconds()
	leaked := timePassed * lb.leakRate
	currentVolume := max(0, b.volume-leaked)
	remaining := max(0, lb.capacity-currentVolume)

	timeToEmpty := currentVolume / lb.leakRate

	return RateLimiterStats{
		Allowed:   currentVolume+1 <= lb.capacity,
		Limit:     int(lb.capacity),
		Remaining: int(remaining),
		ResetAt:   now.Add(time.Duration(timeToEmpty) * time.Second),
	}
}

func max(a, b float64) float64 {
	if a > b {
		return a
	}
	return b
}