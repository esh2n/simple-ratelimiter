package algorithms

import "time"

type RateLimiter interface {
	Allow(key string) bool
	Reset(key string)
	GetStats(key string) RateLimiterStats
}

type RateLimiterStats struct {
	Allowed   bool
	Limit     int
	Remaining int
	ResetAt   time.Time
}