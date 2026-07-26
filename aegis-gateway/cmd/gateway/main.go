package main

import (
	"context"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/aegis-ai/aegis/gateway/internal/config"
	"github.com/aegis-ai/aegis/gateway/internal/providers"
	"github.com/aegis-ai/aegis/gateway/internal/ratelimit"
	"github.com/aegis-ai/aegis/gateway/internal/protocol"
	"github.com/aegis-ai/aegis/gateway/internal/router"
	"github.com/aegis-ai/aegis/gateway/internal/telemetry"
	"github.com/aegis-ai/aegis/gateway/internal/middleware"
	"github.com/rs/zerolog"
)

func main() {
	logger := zerolog.New(os.Stderr).With().Timestamp().Logger()

	cfg := config.Load()
	logger.Info().Str("http_port", cfg.HTTPPort).Str("grpc_port", cfg.GRPCPort).Msg("Starting AEGIS Gateway")

	tp, err := telemetry.Init(cfg)
	if err != nil {
		logger.Warn().Err(err).Msg("Telemetry init failed, continuing without")
	}

	pm := providers.NewManager()

	for _, p := range cfg.Providers {
		pm.Register(&p)
	}
	logger.Info().Int("providers", len(cfg.Providers)).Msg("Providers registered")

	rl := ratelimit.New(cfg.RedisAddr)
	pa := protocol.NewAdapter()
	rt := router.New(pm, rl, &logger)

	mux := http.NewServeMux()
	mux.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		fmt.Fprintf(w, `{"status":"ok","service":"aegis-gateway","uptime_seconds":%d}`, int(time.Now().Sub(startTime).Seconds()))
	})

	mux.HandleFunc("/v1/route", middleware.Chain(
		middleware.Logging(&logger),
		middleware.Tracing(tp),
		middleware.AgentID,
	)(rt.HandleRoute))

	mux.HandleFunc("/v1/chat/completions", middleware.Chain(
		middleware.Logging(&logger),
		middleware.Tracing(tp),
		middleware.AgentID,
	)(rt.HandleProxy))

	mux.HandleFunc("/v1/providers", pm.HandleList)

	srv := &http.Server{
		Addr:         fmt.Sprintf(":%s", cfg.HTTPPort),
		Handler:      mux,
		ReadTimeout:  30 * time.Second,
		WriteTimeout: 60 * time.Second,
		IdleTimeout:  120 * time.Second,
	}

	go func() {
		logger.Info().Str("addr", srv.Addr).Msg("HTTP server listening")
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			logger.Fatal().Err(err).Msg("Server error")
		}
	}()

	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
	<-quit

	logger.Info().Msg("Shutting down gateway...")
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	if err := srv.Shutdown(ctx); err != nil {
		logger.Fatal().Err(err).Msg("Server forced shutdown")
	}
	logger.Info().Msg("Gateway stopped")
}

var startTime = time.Now()
