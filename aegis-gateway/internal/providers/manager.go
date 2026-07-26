package providers

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"math/rand"
	"net/http"
	"strings"
	"sync"
	"time"

	"github.com/aegis-ai/aegis/gateway/internal/config"
)

type ProviderState struct {
	Config     *config.ProviderConfig
	Client     *http.Client
	Healthy    bool
	LastCheck  time.Time
	Failures   int
	Successes  int
	TotalCost  float64
	mu         sync.RWMutex
}

type Manager struct {
	providers map[string]*ProviderState
	mu        sync.RWMutex
}

func NewManager() *Manager {
	return &Manager{
		providers: make(map[string]*ProviderState),
	}
}

func (m *Manager) Register(cfg *config.ProviderConfig) {
	m.mu.Lock()
	defer m.mu.Unlock()

	m.providers[cfg.Name] = &ProviderState{
		Config: cfg,
		Client: &http.Client{
			Timeout: time.Duration(cfg.Timeout) * time.Millisecond,
			Transport: &http.Transport{
				MaxIdleConns:        100,
				MaxIdleConnsPerHost: 20,
				IdleConnTimeout:     90 * time.Second,
			},
		},
		Healthy:   true,
		LastCheck: time.Now(),
	}
}

func (m *Manager) Get(name string) *ProviderState {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return m.providers[name]
}

func (m *Manager) List() []*ProviderState {
	m.mu.RLock()
	defer m.mu.RUnlock()

	result := make([]*ProviderState, 0, len(m.providers))
	for _, p := range m.providers {
		result = append(result, p)
	}
	return result
}

func (m *Manager) SelectProvider(model string) *ProviderState {
	m.mu.RLock()
	defer m.mu.RUnlock()

	type candidate struct {
		provider *ProviderState
		weight   int
	}

	var candidates []candidate
	var totalWeight int

	for _, p := range m.providers {
		p.mu.RLock()
		healthy := p.Healthy
		p.mu.RUnlock()

		if !healthy {
			continue
		}
		if !p.Config.SupportsModel(model) {
			continue
		}

		candidates = append(candidates, candidate{provider: p, weight: p.Config.Weight})
		totalWeight += p.Config.Weight
	}

	if len(candidates) == 0 {
		return nil
	}

	// Weighted random selection
	r := rand.Intn(totalWeight)
	for _, c := range candidates {
		r -= c.weight
		if r < 0 {
			return c.provider
		}
	}

	return candidates[0].provider
}

func (m *Manager) HandleList(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")

	type providerInfo struct {
		Name      string   `json:"name"`
		Healthy   bool     `json:"healthy"`
		Models    []string `json:"models"`
		Failures  int      `json:"failures"`
		Successes int      `json:"successes"`
		TotalCost float64  `json:"total_cost_usd"`
	}

	var info []providerInfo
	for _, p := range m.List() {
		p.mu.RLock()
		info = append(info, providerInfo{
			Name:      p.Config.Name,
			Healthy:   p.Healthy,
			Models:    p.Config.Models,
			Failures:  p.Failures,
			Successes: p.Successes,
			TotalCost: p.TotalCost,
		})
		p.mu.RUnlock()
	}

	json.NewEncoder(w).Encode(info)
}

func (p *ProviderState) Forward(ctx context.Context, path string, headers http.Header, body io.Reader) (*http.Response, error) {
	url := strings.TrimRight(p.Config.BaseURL, "/") + "/" + strings.TrimLeft(path, "/")

	req, err := http.NewRequestWithContext(ctx, "POST", url, body)
	if err != nil {
		return nil, fmt.Errorf("create request: %w", err)
	}

	// Copy headers
	for k, v := range headers {
		req.Header[k] = v
	}

	// Set API key
	if key := p.Config.APIKey(); key != "" {
		req.Header.Set("Authorization", "Bearer "+key)
	}
	req.Header.Set("Content-Type", "application/json")

	resp, err := p.Client.Do(req)
	if err != nil {
		p.recordFailure()
		return nil, fmt.Errorf("provider request: %w", err)
	}

	p.recordSuccess()
	return resp, nil
}

func (p *ProviderState) recordFailure() {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.Failures++
	if p.Failures >= 5 && time.Since(p.LastCheck) > time.Minute {
		p.Healthy = false
		p.LastCheck = time.Now()
	}
}

func (p *ProviderState) recordSuccess() {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.Successes++
	p.Failures = 0
	p.Healthy = true
	p.LastCheck = time.Now()
}

func (p *ProviderState) RecordCost(usd float64) {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.TotalCost += usd
}

type ChatRequest struct {
	Model    string          `json:"model"`
	Messages []ChatMessage   `json:"messages"`
	Stream   bool            `json:"stream,omitempty"`
	MaxTokens int            `json:"max_tokens,omitempty"`
	Temperature float64      `json:"temperature,omitempty"`
}

type ChatMessage struct {
	Role    string `json:"role"`
	Content string `json:"content"`
}

type ChatResponse struct {
	ID      string   `json:"id"`
	Model   string   `json:"model"`
	Choices []Choice `json:"choices"`
	Usage   Usage    `json:"usage"`
}

type Choice struct {
	Index   int         `json:"index"`
	Message ChatMessage `json:"message"`
}

type Usage struct {
	PromptTokens     int `json:"prompt_tokens"`
	CompletionTokens int `json:"completion_tokens"`
	TotalTokens      int `json:"total_tokens"`
}

func EstimateCost(model string, usage Usage) float64 {
	// Cost per 1K tokens (approximate)
	promptRate := 0.01
	completionRate := 0.03

	switch {
	case strings.HasPrefix(model, "gpt-4"):
		promptRate = 0.03
		completionRate = 0.06
	case strings.HasPrefix(model, "claude-3-opus"):
		promptRate = 0.015
		completionRate = 0.075
	case strings.HasPrefix(model, "claude-3-sonnet"):
		promptRate = 0.003
		completionRate = 0.015
	case strings.HasPrefix(model, "claude-4"):
		promptRate = 0.015
		completionRate = 0.075
	case strings.HasPrefix(model, "gemini"):
		promptRate = 0.002
		completionRate = 0.008
	case strings.HasPrefix(model, "mistral-large"):
		promptRate = 0.004
		completionRate = 0.012
	case strings.HasPrefix(model, "mistral-medium"):
		promptRate = 0.002
		completionRate = 0.006
	}

	promptCost := (float64(usage.PromptTokens) / 1000) * promptRate
	completionCost := (float64(usage.CompletionTokens) / 1000) * completionRate
	return promptCost + completionCost
}
