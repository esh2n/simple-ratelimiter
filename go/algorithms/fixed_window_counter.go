package algorithms

import (
	"sync"
	"time"
)

type window struct {
	count     int
	startTime time.Time
}

type FixedWindowCounter struct {
	mu         sync.RWMutex
	windows    map[string]*window
	limit      int
	windowSize time.Duration
}

func NewFixedWindowCounter(limit int, windowSize time.Duration) *FixedWindowCounter {
	return &FixedWindowCounter{
		windows:    make(map[string]*window),
		limit:      limit,
		windowSize: windowSize,
	}
}

func (fwc *FixedWindowCounter) Allow(key string) bool {
	fwc.mu.Lock()
	defer fwc.mu.Unlock()

	now := time.Now()
	currentWindow := now.Truncate(fwc.windowSize)

	w, exists := fwc.windows[key]
	if !exists || w.startTime != currentWindow {
		w = &window{
			count:     0,
			startTime: currentWindow,
		}
		fwc.windows[key] = w
	}

	if w.count < fwc.limit {
		w.count++
		return true
	}

	return false
}

func (fwc *FixedWindowCounter) Reset(key string) {
	fwc.mu.Lock()
	defer fwc.mu.Unlock()
	delete(fwc.windows, key)
}

func (fwc *FixedWindowCounter) GetStats(key string) RateLimiterStats {
	fwc.mu.RLock()
	defer fwc.mu.RUnlock()

	now := time.Now()
	currentWindow := now.Truncate(fwc.windowSize)

	w, exists := fwc.windows[key]
	if !exists || w.startTime != currentWindow {
		return RateLimiterStats{
			Allowed:   true,
			Limit:     fwc.limit,
			Remaining: fwc.limit,
			ResetAt:   currentWindow.Add(fwc.windowSize),
		}
	}

	remaining := fwc.limit - w.count
	if remaining < 0 {
		remaining = 0
	}

	return RateLimiterStats{
		Allowed:   w.count < fwc.limit,
		Limit:     fwc.limit,
		Remaining: remaining,
		ResetAt:   currentWindow.Add(fwc.windowSize),
	}
}