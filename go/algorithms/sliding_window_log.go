package algorithms

import (
	"sync"
	"time"
)

type SlidingWindowLog struct {
	mu         sync.RWMutex
	logs       map[string][]time.Time
	limit      int
	windowSize time.Duration
}

func NewSlidingWindowLog(limit int, windowSize time.Duration) *SlidingWindowLog {
	return &SlidingWindowLog{
		logs:       make(map[string][]time.Time),
		limit:      limit,
		windowSize: windowSize,
	}
}

func (swl *SlidingWindowLog) Allow(key string) bool {
	swl.mu.Lock()
	defer swl.mu.Unlock()

	now := time.Now()
	timestamps, exists := swl.logs[key]
	if !exists {
		timestamps = []time.Time{}
	}

	// Remove expired timestamps
	validTimestamps := make([]time.Time, 0, len(timestamps))
	cutoff := now.Add(-swl.windowSize)
	for _, ts := range timestamps {
		if ts.After(cutoff) {
			validTimestamps = append(validTimestamps, ts)
		}
	}

	if len(validTimestamps) < swl.limit {
		validTimestamps = append(validTimestamps, now)
		swl.logs[key] = validTimestamps
		return true
	}

	swl.logs[key] = validTimestamps
	return false
}

func (swl *SlidingWindowLog) Reset(key string) {
	swl.mu.Lock()
	defer swl.mu.Unlock()
	delete(swl.logs, key)
}

func (swl *SlidingWindowLog) GetStats(key string) RateLimiterStats {
	swl.mu.RLock()
	defer swl.mu.RUnlock()

	now := time.Now()
	timestamps, exists := swl.logs[key]
	if !exists {
		return RateLimiterStats{
			Allowed:   true,
			Limit:     swl.limit,
			Remaining: swl.limit,
			ResetAt:   now.Add(swl.windowSize),
		}
	}

	// Count valid timestamps
	validCount := 0
	cutoff := now.Add(-swl.windowSize)
	var oldestValid time.Time
	for _, ts := range timestamps {
		if ts.After(cutoff) {
			validCount++
			if oldestValid.IsZero() || ts.Before(oldestValid) {
				oldestValid = ts
			}
		}
	}

	remaining := swl.limit - validCount
	if remaining < 0 {
		remaining = 0
	}

	var resetAt time.Time
	if validCount == 0 {
		resetAt = now.Add(swl.windowSize)
	} else {
		resetAt = oldestValid.Add(swl.windowSize)
	}

	return RateLimiterStats{
		Allowed:   validCount < swl.limit,
		Limit:     swl.limit,
		Remaining: remaining,
		ResetAt:   resetAt,
	}
}