package config

import (
	"os"
	"strconv"
	"strings"
)

type ProviderConfig struct {
	Name      string   `yaml:"name"`
	BaseURL   string   `yaml:"base_url"`
	APIKeyEnv string   `yaml:"api_key_env"`
	Models    []string `yaml:"models"`
	Weight    int      `yaml:"weight"`
	Timeout   int      `yaml:"timeout_ms"`
	MaxRetries int     `yaml:"max_retries"`
}

type Config struct {
	HTTPPort  string
	GRPCPort  string
	RedisAddr string
	OTLPEndpoint string
	LogLevel  string
	Providers []ProviderConfig
}

func Load() *Config {
	return &Config{
		HTTPPort:     getEnv("AEGIS_GATEWAY_PORT", "8000"),
		GRPCPort:     getEnv("AEGIS_GATEWAY_GRPC_PORT", "8001"),
		RedisAddr:    getEnv("AEGIS_REDIS_ADDR", "localhost:6379"),
		OTLPEndpoint: getEnv("AEGIS_OTLP_ENDPOINT", "localhost:4317"),
		LogLevel:     getEnv("AEGIS_LOG_LEVEL", "info"),
		Providers:    defaultProviders(),
	}
}

func defaultProviders() []ProviderConfig {
	return []ProviderConfig{
		{
			Name:      "openai",
			BaseURL:   "https://api.openai.com/v1",
			APIKeyEnv: "AEGIS_OPENAI_API_KEY",
			Models:    []string{"gpt-4", "gpt-4o", "gpt-4o-mini", "o1", "o1-mini", "o3-mini"},
			Weight:    50,
			Timeout:   30000,
			MaxRetries: 3,
		},
		{
			Name:      "anthropic",
			BaseURL:   "https://api.anthropic.com/v1",
			APIKeyEnv: "AEGIS_ANTHROPIC_API_KEY",
			Models:    []string{"claude-3", "claude-3-opus", "claude-3-sonnet", "claude-4", "claude-4-opus"},
			Weight:    30,
			Timeout:   60000,
			MaxRetries: 3,
		},
		{
			Name:      "google",
			BaseURL:   "https://generativelanguage.googleapis.com/v1",
			APIKeyEnv: "AEGIS_GOOGLE_API_KEY",
			Models:    []string{"gemini-1.5", "gemini-2.0", "gemini-2.5"},
			Weight:    10,
			Timeout:   30000,
			MaxRetries: 2,
		},
		{
			Name:      "mistral",
			BaseURL:   "https://api.mistral.ai/v1",
			APIKeyEnv: "AEGIS_MISTRAL_API_KEY",
			Models:    []string{"mistral-large", "mistral-medium", "mistral-small", "codestral"},
			Weight:    5,
			Timeout:   30000,
			MaxRetries: 2,
		},
		{
			Name:      "ollama",
			BaseURL:   "http://localhost:11434",
			APIKeyEnv: "",
			Models:    []string{"llama3", "llama3.1", "llama3.2", "mistral", "codellama", "phi", "gemma", "qwen"},
			Weight:    3,
			Timeout:   120000,
			MaxRetries: 1,
		},
		{
			Name:      "openrouter",
			BaseURL:   "https://openrouter.ai/api/v1",
			APIKeyEnv: "AEGIS_OPENROUTER_API_KEY",
			Models:    []string{"*"},
			Weight:    2,
			Timeout:   60000,
			MaxRetries: 2,
		},
	}
}

func getEnv(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}

func getEnvInt(key string, fallback int) int {
	if v := os.Getenv(key); v != "" {
		if i, err := strconv.Atoi(v); err == nil {
			return i
		}
	}
	return fallback
}

func (p *ProviderConfig) APIKey() string {
	if p.APIKeyEnv == "" {
		return ""
	}
	return os.Getenv(p.APIKeyEnv)
}

func (p *ProviderConfig) SupportsModel(model string) bool {
	for _, m := range p.Models {
		if m == "*" {
			return true
		}
		if strings.HasPrefix(model, m) {
			return true
		}
	}
	return false
}
