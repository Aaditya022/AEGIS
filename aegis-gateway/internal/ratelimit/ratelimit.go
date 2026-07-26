package ratelimit

import (
	"context"
	"sync"
	"time"

	"github.com/redis/go-redis/v9"
)

type RateLimiter struct {
	mu          sync.Mutex
	tokens      map[string]float64
	lastRefill  map[string]time.Time
	rate        float64
	burst       int
	redisClient *redis.Client
	useRedis    bool
}

func New(redisAddr string) *RateLimiter {
	rl := &RateLimiter{
		tokens:     make(map[string]float64),
		lastRefill: make(map[string]time.Time),
		rate:       100.0,
		burst:      200,
	}

	if redisAddr != "" {
		rl.redisClient = redis.NewClient(&redis.Options{
			Addr:         redisAddr,
			DialTimeout:  2 * time.Second,
			ReadTimeout:  1 * time.Second,
			WriteTimeout: 1 * time.Second,
			PoolSize:     10,
		})
		ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
		defer cancel()
		if err := rl.redisClient.Ping(ctx).Err(); err == nil {
			rl.useRedis = true
		}
	}

	return rl
}

func (rl *RateLimiter) Allow(key string) bool {
	if rl.useRedis {
		return rl.allowRedis(key)
	}
	return rl.allowLocal(key)
}

func (rl *RateLimiter) allowRedis(key string) bool {
	ctx := context.Background()
	now := time.Now().Unix()
	window := int64(1)

	count, err := rl.redisClient.Get(ctx, "ratelimit:"+key).Int64()
	if err == redis.Nil {
		rl.redisClient.Set(ctx, "ratelimit:"+key, 1, time.Duration(window)*time.Second)
		return true
	} else if err != nil {
		return rl.allowLocal(key)
	}

	if count >= int64(rl.burst) {
		return false
	}

	rl.redisClient.Incr(ctx, "ratelimit:"+key)
	return true
}

func (rl *RateLimiter) allowLocal(key string) bool {
	rl.mu.Lock()
	defer rl.mu.Unlock()

	now := time.Now()
	lastRefill, exists := rl.lastRefill[key]
	if !exists {
		rl.tokens[key] = float64(rl.burst)
		rl.lastRefill[key] = now
	}

	elapsed := now.Sub(lastRefill).Seconds()
	rl.tokens[key] = rl.tokens[key] + elapsed*rl.rate
	if rl.tokens[key] > float64(rl.burst) {
		rl.tokens[key] = float64(rl.burst)
	}

	if rl.tokens[key] >= 1.0 {
		rl.tokens[key]--
		rl.lastRefill[key] = now
		return true
	}

	return false
}

func (rl *RateLimiter) Close() error {
	if rl.redisClient != nil {
		return rl.redisClient.Close()
	}
	return nil
}
