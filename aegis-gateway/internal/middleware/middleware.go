package middleware

import (
	"context"
	"net/http"
	"time"

	"github.com/rs/zerolog"
	"go.opentelemetry.io/otel"
	"go.opentelemetry.io/otel/attribute"
	"go.opentelemetry.io/otel/trace"
)

type contextKey string

const AgentIDKey contextKey = "agent_id"

type Middleware func(http.HandlerFunc) http.HandlerFunc

func Chain(middlewares ...Middleware) Middleware {
	return func(next http.HandlerFunc) http.HandlerFunc {
		for i := len(middlewares) - 1; i >= 0; i-- {
			next = middlewares[i](next)
		}
		return next
	}
}

func Logging(logger *zerolog.Logger) Middleware {
	return func(next http.HandlerFunc) http.HandlerFunc {
		return func(w http.ResponseWriter, r *http.Request) {
			start := time.Now()

			lrw := &loggingResponseWriter{ResponseWriter: w, statusCode: http.StatusOK}
			next(lrw, r)

			logger.Info().
				Str("method", r.Method).
				Str("path", r.URL.Path).
				Int("status", lrw.statusCode).
				Str("agent_id", r.Header.Get("X-AEGIS-Agent-ID")).
				Dur("latency", time.Since(start)).
				Msg("Request")
		}
	}
}

type loggingResponseWriter struct {
	http.ResponseWriter
	statusCode int
}

func (lrw *loggingResponseWriter) WriteHeader(code int) {
	lrw.statusCode = code
	lrw.ResponseWriter.WriteHeader(code)
}

func Tracing(tp trace.TracerProvider) Middleware {
	return func(next http.HandlerFunc) http.HandlerFunc {
		return func(w http.ResponseWriter, r *http.Request) {
			if tp == nil {
				next(w, r)
				return
			}

			tracer := tp.Tracer("aegis-gateway")
			ctx, span := tracer.Start(r.Context(), r.URL.Path,
				trace.WithAttributes(
					attribute.String("http.method", r.Method),
					attribute.String("http.path", r.URL.Path),
					attribute.String("agent_id", r.Header.Get("X-AEGIS-Agent-ID")),
				),
			)
			defer span.End()

			next(w, r.WithContext(ctx))
		}
	}
}

func AgentID(next http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		agentID := r.Header.Get("X-AEGIS-Agent-ID")
		if agentID == "" {
			agentID = r.Header.Get("X-Agent-ID")
		}
		if agentID == "" {
			agentID = "anonymous"
		}
		ctx := context.WithValue(r.Context(), AgentIDKey, agentID)
		next(w, r.WithContext(ctx))
	}
}
