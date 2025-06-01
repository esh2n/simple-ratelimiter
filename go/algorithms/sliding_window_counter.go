package algorithms

import (
	"math"
	"sync"
	"time"
)

type windowData struct {
	prevCount       int
	currCount       int
	prevWindowStart time.Time
	currWindowStart time.Time
}

type SlidingWindowCounter struct {
	mu         sync.RWMutex
	windows    map[string]*windowData
	limit      int
	windowSize time.Duration
}

func NewSlidingWindowCounter(limit int, windowSize time.Duration) *SlidingWindowCounter {
	return &SlidingWindowCounter{
		windows:    make(map[string]*windowData),
		limit:      limit,
		windowSize: windowSize,
	}
}

func (swc *SlidingWindowCounter) Allow(key string) bool {
	swc.mu.Lock()
	defer swc.mu.Unlock()

	now := time.Now()
	currentWindowStart := now.Truncate(swc.windowSize)

	data, exists := swc.windows[key]
	if !exists {
		data = &windowData{
			prevCount:       0,
			currCount:       0,
			prevWindowStart: currentWindowStart.Add(-swc.windowSize),
			currWindowStart: currentWindowStart,
		}
		swc.windows[key] = data
	}

	// Check if we need to slide the window
	if data.currWindowStart.Before(currentWindowStart) {
		data.prevCount = data.currCount
		data.prevWindowStart = data.currWindowStart
		data.currCount = 0
		data.currWindowStart = currentWindowStart
	}

	// Calculate the weighted count
	windowProgress := float64(now.Sub(currentWindowStart)) / float64(swc.windowSize)
	prevWindowWeight := 1.0 - windowProgress
	estimatedCount := float64(data.prevCount)*prevWindowWeight + float64(data.currCount)

	if estimatedCount < float64(swc.limit) {
		data.currCount++
		return true
	}

	return false
}

func (swc *SlidingWindowCounter) Reset(key string) {
	swc.mu.Lock()
	defer swc.mu.Unlock()
	delete(swc.windows, key)
}

func (swc *SlidingWindowCounter) GetStats(key string) RateLimiterStats {
	swc.mu.RLock()
	defer swc.mu.RUnlock()

	now := time.Now()
	currentWindowStart := now.Truncate(swc.windowSize)

	data, exists := swc.windows[key]
	if !exists || data.currWindowStart.Before(currentWindowStart) {
		return RateLimiterStats{
			Allowed:   true,
			Limit:     swc.limit,
			Remaining: swc.limit,
			ResetAt:   currentWindowStart.Add(swc.windowSize),
		}
	}

	// Calculate the weighted count
	windowProgress := float64(now.Sub(currentWindowStart)) / float64(swc.windowSize)
	prevWindowWeight := 1.0 - windowProgress
	estimatedCount := float64(data.prevCount)*prevWindowWeight + float64(data.currCount)
	remaining := int(math.Max(0, float64(swc.limit)-estimatedCount))

	return RateLimiterStats{
		Allowed:   estimatedCount < float64(swc.limit),
		Limit:     swc.limit,
		Remaining: remaining,
		ResetAt:   currentWindowStart.Add(swc.windowSize),
	}
}