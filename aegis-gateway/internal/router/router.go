package router

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"

	"github.com/aegis-ai/aegis/gateway/internal/providers"
	"github.com/aegis-ai/aegis/gateway/internal/protocol"
	"github.com/aegis-ai/aegis/gateway/internal/ratelimit"
	"github.com/rs/zerolog"
)

type Router struct {
	providers *providers.Manager
	ratelimit *ratelimit.RateLimiter
	logger    *zerolog.Logger
}

func New(pm *providers.Manager, rl *ratelimit.RateLimiter, logger *zerolog.Logger) *Router {
	return &Router{
		providers: pm,
		ratelimit: rl,
		logger:    logger,
	}
}

func (rt *Router) HandleRoute(w http.ResponseWriter, r *http.Request) {
	agentID := r.Header.Get("X-AEGIS-Agent-ID")
	model := r.URL.Query().Get("model")
	if model == "" {
		var body struct {
			Model string `json:"model"`
		}
		if err := json.NewDecoder(r.Body).Decode(&body); err == nil {
			model = body.Model
		}
	}

	if model == "" {
		http.Error(w, `{"error":"missing model parameter"}`, http.StatusBadRequest)
		return
	}

	provider := rt.providers.SelectProvider(model)
	if provider == nil {
		http.Error(w, fmt.Sprintf(`{"error":"no healthy provider for model %s"}`, model), http.StatusServiceUnavailable)
		return
	}

	// Rate limit check
	rate := 50.0 // requests per second
	burst := 100
	allowed, wait := rt.ratelimit.Allow(r.Context(), agentID, rate, burst)
	if !allowed {
		w.Header().Set("Retry-After", fmt.Sprintf("%.0f", wait.Seconds()))
		http.Error(w, `{"error":"rate limit exceeded"}`, http.StatusTooManyRequests)
		return
	}

	resp := map[string]interface{}{
		"provider": provider.Config.Name,
		"model":    model,
		"endpoint": provider.Config.BaseURL,
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(resp)

	rt.logger.Info().
		Str("agent_id", agentID).
		Str("model", model).
		Str("provider", provider.Config.Name).
		Msg("Route selected")
}

func (rt *Router) HandleProxy(w http.ResponseWriter, r *http.Request) {
	agentID := r.Header.Get("X-AEGIS-Agent-ID")
	start := time.Now()

	body, err := io.ReadAll(r.Body)
	if err != nil {
		http.Error(w, `{"error":"cannot read body"}`, http.StatusBadRequest)
		return
	}

	normalized, err := protocol.DetectAndNormalize(body, flattenHeaders(r.Header), r.URL.Path)
	if err != nil {
		http.Error(w, fmt.Sprintf(`{"error":"parse error: %s"}`, err.Error()), http.StatusBadRequest)
		return
	}

	model := normalized.Model
	if model == "" {
		http.Error(w, `{"error":"model is required"}`, http.StatusBadRequest)
		return
	}

	provider := rt.providers.SelectProvider(model)
	if provider == nil {
		http.Error(w, fmt.Sprintf(`{"error":"no healthy provider for %s"}`, model), http.StatusServiceUnavailable)
		return
	}

	// Rate limit
	allowed, wait := rt.ratelimit.Allow(r.Context(), agentID, 50.0, 100)
	if !allowed {
		w.Header().Set("Retry-After", fmt.Sprintf("%.0f", wait.Seconds()))
		http.Error(w, `{"error":"rate limit exceeded"}`, http.StatusTooManyRequests)
		return
	}

	// Convert to provider-native format
	var providerBody []byte
	switch provider.Config.Name {
	case "anthropic":
		providerBody, err = protocol.NewAdapter().ConvertToAnthropic(normalized)
	default:
		providerBody, err = protocol.NewAdapter().ConvertToOpenAI(normalized)
	}
	if err != nil {
		http.Error(w, fmt.Sprintf(`{"error":"conversion error: %s"}`, err.Error()), http.StatusInternalServerError)
		return
	}

	// Forward to provider
	path := r.URL.Path
	if provider.Config.Name == "anthropic" {
		path = "/v1/messages"
	}

	resp, err := provider.Forward(r.Context(), path, r.Header, bytes.NewReader(providerBody))
	if err != nil {
		rt.logger.Error().Err(err).Str("provider", provider.Config.Name).Msg("Provider request failed")
		http.Error(w, fmt.Sprintf(`{"error":"provider error: %s"}`, err.Error()), http.StatusBadGateway)
		return
	}
	defer resp.Body.Close()

	// Record cost
	respBody, _ := io.ReadAll(resp.Body)
	var chatResp providers.ChatResponse
	if err := json.Unmarshal(respBody, &chatResp); err == nil {
		cost := providers.EstimateCost(model, chatResp.Usage)
		provider.RecordCost(cost)
		rt.logger.Info().
			Str("agent_id", agentID).
			Str("model", model).
			Str("provider", provider.Config.Name).
			Int("tokens", chatResp.Usage.TotalTokens).
			Float64("cost", cost).
			Dur("latency", time.Since(start)).
			Msg("Request completed")
	}

	// Convert response to OpenAI-compatible format
	converted, err := protocol.ConvertResponse(provider.Config.Name, respBody)
	if err != nil {
		http.Error(w, fmt.Sprintf(`{"error":"response conversion error: %s"}`, err.Error()), http.StatusInternalServerError)
		return
	}

	for k, v := range resp.Header {
		w.Header()[k] = v
	}
	w.Header().Set("X-AEGIS-Provider", provider.Config.Name)
	w.Header().Set("X-AEGIS-Model", model)
	w.WriteHeader(resp.StatusCode)
	w.Write(converted)
}

func flattenHeaders(h http.Header) map[string]string {
	result := make(map[string]string)
	for k, v := range h {
		if len(v) > 0 {
			result[k] = v[0]
		}
	}
	return result
}
