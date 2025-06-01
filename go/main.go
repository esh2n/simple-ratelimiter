package main

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"strconv"
	"strings"
	"time"

	"github.com/esh2n/simple-ratelimiter/go/algorithms"
	"github.com/gorilla/mux"
)

var rateLimiters = map[string]algorithms.RateLimiter{
	"token-bucket":          algorithms.NewTokenBucket(10, time.Second, 1),
	"leaky-bucket":          algorithms.NewLeakyBucket(10, 10),
	"fixed-window":          algorithms.NewFixedWindowCounter(10, 60*time.Second),
	"sliding-window-log":    algorithms.NewSlidingWindowLog(10, 60*time.Second),
	"sliding-window-counter": algorithms.NewSlidingWindowCounter(10, 60*time.Second),
}

func rateLimitMiddleware(algorithm string) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			limiter, exists := rateLimiters[algorithm]
			if !exists {
				http.Error(w, "Invalid rate limiter algorithm", http.StatusInternalServerError)
				return
			}

			clientID := getClientID(r)
			key := fmt.Sprintf("%s:%s", algorithm, clientID)

			allowed := limiter.Allow(key)
			stats := limiter.GetStats(key)

			w.Header().Set("X-RateLimit-Limit", strconv.Itoa(stats.Limit))
			w.Header().Set("X-RateLimit-Remaining", strconv.Itoa(stats.Remaining))
			w.Header().Set("X-RateLimit-Reset", stats.ResetAt.Format(time.RFC3339))

			if !allowed {
				retryAfter := int(stats.ResetAt.Sub(time.Now()).Seconds())
				if retryAfter < 0 {
					retryAfter = 0
				}

				w.Header().Set("Content-Type", "application/json")
				w.WriteHeader(http.StatusTooManyRequests)
				json.NewEncoder(w).Encode(map[string]interface{}{
					"error":      "Too Many Requests",
					"retryAfter": retryAfter,
				})
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}

func getClientID(r *http.Request) string {
	forwarded := r.Header.Get("X-Forwarded-For")
	if forwarded != "" {
		parts := strings.Split(forwarded, ",")
		return strings.TrimSpace(parts[0])
	}
	
	ip := r.RemoteAddr
	if colonIndex := strings.LastIndex(ip, ":"); colonIndex != -1 {
		ip = ip[:colonIndex]
	}
	
	if ip == "" {
		ip = "unknown"
	}
	
	return ip
}

func dataHandler(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	algorithm := vars["algorithm"]

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"message":   "Success",
		"algorithm": algorithm,
		"timestamp": time.Now().Format(time.RFC3339),
	})
}

func statsHandler(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	algorithm := vars["algorithm"]

	limiter, exists := rateLimiters[algorithm]
	if !exists {
		http.Error(w, "Algorithm not found", http.StatusNotFound)
		return
	}

	clientID := getClientID(r)
	key := fmt.Sprintf("%s:%s", algorithm, clientID)
	stats := limiter.GetStats(key)

	algorithms := make([]string, 0, len(rateLimiters))
	for k := range rateLimiters {
		algorithms = append(algorithms, k)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"algorithm":           algorithm,
		"stats":               stats,
		"availableAlgorithms": algorithms,
	})
}

func algorithmsHandler(w http.ResponseWriter, r *http.Request) {
	algorithms := make([]string, 0, len(rateLimiters))
	for k := range rateLimiters {
		algorithms = append(algorithms, k)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"algorithms": algorithms,
		"endpoints": map[string]string{
			"data":  "/api/data/{algorithm}",
			"stats": "/api/stats/{algorithm}",
		},
	})
}

func main() {
	r := mux.NewRouter()

	for algorithm := range rateLimiters {
		r.HandleFunc("/api/data/"+algorithm, dataHandler).
			Methods("GET").
			Subrouter().
			Use(rateLimitMiddleware(algorithm))
	}

	r.HandleFunc("/api/stats/{algorithm}", statsHandler).Methods("GET")
	r.HandleFunc("/api/algorithms", algorithmsHandler).Methods("GET")

	port := os.Getenv("PORT")
	if port == "" {
		port = "3001"
	}

	algorithms := make([]string, 0, len(rateLimiters))
	for k := range rateLimiters {
		algorithms = append(algorithms, k)
	}

	fmt.Printf("Go Rate Limiter Server running on port %s\n", port)
	fmt.Printf("Available algorithms: %s\n", strings.Join(algorithms, ", "))

	log.Fatal(http.ListenAndServe(":"+port, r))
}