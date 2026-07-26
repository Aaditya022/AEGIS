package protocol

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"strings"
)

type ProtocolType string

const (
	ProtocolHTTP   ProtocolType = "http"
	ProtocolGRPC   ProtocolType = "grpc"
	ProtocolMCP    ProtocolType = "mcp"    // Model Context Protocol
	ProtocolA2A    ProtocolType = "a2a"    // Agent-to-Agent
	ProtocolACP    ProtocolType = "acp"    // Agent Communication Protocol
	ProtocolANP    ProtocolType = "anp"    // Agent Network Protocol
	ProtocolOpenAI ProtocolType = "openai"
	ProtocolAnthropic ProtocolType = "anthropic"
)

type Adapter struct{}

func NewAdapter() *Adapter {
	return &Adapter{}
}

func (a *Adapter) DetectProtocol(headers map[string]string, path string, body []byte) ProtocolType {
	if contentType := headers["Content-Type"]; contentType == "application/grpc" {
		return ProtocolGRPC
	}

	if v := headers["MCP-Version"]; v != "" {
		return ProtocolMCP
	}
	if v := headers["X-MCP-Version"]; v != "" {
		return ProtocolMCP
	}

	if v := headers["A2A-Version"]; v != "" {
		return ProtocolA2A
	}
	if v := headers["X-A2A-Version"]; v != "" {
		return ProtocolA2A
	}

	if v := headers["ACP-Version"]; v != "" {
		return ProtocolACP
	}

	if strings.Contains(path, "/v1/chat/completions") {
		return ProtocolOpenAI
	}
	if strings.Contains(path, "/v1/messages") {
		return ProtocolAnthropic
	}

	return ProtocolHTTP
}

type NormalizedRequest struct {
	Model       string            `json:"model"`
	Messages    []NormalizedMessage `json:"messages"`
	Stream      bool              `json:"stream"`
	MaxTokens   int               `json:"max_tokens"`
	Temperature float64           `json:"temperature"`
	Tools       []Tool            `json:"tools,omitempty"`
	Raw         json.RawMessage   `json:"-"`
}

type NormalizedMessage struct {
	Role    string `json:"role"`
	Content string `json:"content"`
}

type Tool struct {
	Name        string          `json:"name"`
	Description string          `json:"description"`
	Schema      json.RawMessage `json:"schema"`
}

func (a *Adapter) Normalize(body []byte, sourceProtocol ProtocolType) (*NormalizedRequest, error) {
	switch sourceProtocol {
	case ProtocolOpenAI:
		return a.normalizeOpenAI(body)
	case ProtocolAnthropic:
		return a.normalizeAnthropic(body)
	case ProtocolMCP:
		return a.normalizeMCP(body)
	case ProtocolA2A:
		return a.normalizeA2A(body)
	default:
		return a.normalizeOpenAI(body)
	}
}

func (a *Adapter) normalizeOpenAI(body []byte) (*NormalizedRequest, error) {
	var req struct {
		Model       string    `json:"model"`
		Messages    []struct {
			Role    string `json:"role"`
			Content string `json:"content"`
		} `json:"messages"`
		Stream      bool      `json:"stream"`
		MaxTokens   int       `json:"max_tokens"`
		Temperature float64   `json:"temperature"`
		Tools       []Tool    `json:"tools,omitempty"`
	}

	if err := json.Unmarshal(body, &req); err != nil {
		return nil, fmt.Errorf("openai parse: %w", err)
	}

	nr := &NormalizedRequest{
		Model:       req.Model,
		Stream:      req.Stream,
		MaxTokens:   req.MaxTokens,
		Temperature: req.Temperature,
		Tools:       req.Tools,
		Raw:         body,
	}
	for _, m := range req.Messages {
		nr.Messages = append(nr.Messages, NormalizedMessage{Role: m.Role, Content: m.Content})
	}
	return nr, nil
}

func (a *Adapter) normalizeAnthropic(body []byte) (*NormalizedRequest, error) {
	var req struct {
		Model       string  `json:"model"`
		Messages    []struct {
			Role    string `json:"role"`
			Content string `json:"content"`
		} `json:"messages"`
		MaxTokens   int     `json:"max_tokens"`
		Temperature float64 `json:"temperature"`
		Stream      bool    `json:"stream"`
	}

	if err := json.Unmarshal(body, &req); err != nil {
		return nil, fmt.Errorf("anthropic parse: %w", err)
	}

	nr := &NormalizedRequest{
		Model:       req.Model,
		MaxTokens:   req.MaxTokens,
		Temperature: req.Temperature,
		Stream:      req.Stream,
		Raw:         body,
	}
	for _, m := range req.Messages {
		nr.Messages = append(nr.Messages, NormalizedMessage{Role: m.Role, Content: m.Content})
	}
	return nr, nil
}

func (a *Adapter) normalizeMCP(body []byte) (*NormalizedRequest, error) {
	var req struct {
		Method string `json:"method"`
		Params struct {
			Messages []struct {
				Role    string `json:"role"`
				Content string `json:"content"`
			} `json:"messages"`
			MaxTokens int `json:"maxTokens"`
		} `json:"params"`
	}

	if err := json.Unmarshal(body, &req); err != nil {
		return nil, fmt.Errorf("mcp parse: %w", err)
	}

	nr := &NormalizedRequest{
		Model:     "mcp-model",
		MaxTokens: req.Params.MaxTokens,
		Raw:       body,
	}
	for _, m := range req.Params.Messages {
		nr.Messages = append(nr.Messages, NormalizedMessage{Role: m.Role, Content: m.Content})
	}
	return nr, nil
}

func (a *Adapter) normalizeA2A(body []byte) (*NormalizedRequest, error) {
	var req struct {
		JSONRPC string `json:"jsonrpc"`
		Method  string `json:"method"`
		Params  struct {
			Messages []struct {
				Role    string `json:"role"`
				Content string `json:"content"`
			} `json:"messages"`
		} `json:"params"`
	}

	if err := json.Unmarshal(body, &req); err != nil {
		return nil, fmt.Errorf("a2a parse: %w", err)
	}

	nr := &NormalizedRequest{
		Model: "a2a-model",
		Raw:   body,
	}
	for _, m := range req.Params.Messages {
		nr.Messages = append(nr.Messages, NormalizedMessage{Role: m.Role, Content: m.Content})
	}
	return nr, nil
}

func (a *Adapter) ConvertToOpenAI(nr *NormalizedRequest) ([]byte, error) {
	req := map[string]interface{}{
		"model":       nr.Model,
		"messages":    nr.Messages,
		"stream":      nr.Stream,
		"max_tokens":  nr.MaxTokens,
		"temperature": nr.Temperature,
	}
	if nr.Tools != nil {
		req["tools"] = nr.Tools
	}
	return json.Marshal(req)
}

func (a *Adapter) ConvertToAnthropic(nr *NormalizedRequest) ([]byte, error) {
	req := map[string]interface{}{
		"model":       nr.Model,
		"messages":    nr.Messages,
		"max_tokens":  nr.MaxTokens,
		"temperature": nr.Temperature,
		"stream":      nr.Stream,
	}
	return json.Marshal(req)
}

func DetectAndNormalize(body []byte, headers map[string]string, path string) (*NormalizedRequest, error) {
	adapter := NewAdapter()
	proto := adapter.DetectProtocol(headers, path, body)
	return adapter.Normalize(body, proto)
}

type ResponseWriter interface {
	Write(data []byte) (int, error)
	Header() map[string][]string
}

func CopyResponse(dst io.Writer, src io.Reader) error {
	_, err := io.Copy(dst, src)
	return err
}

func ConvertResponse(providerName string, body []byte) ([]byte, error) {
	if providerName == "openai" {
		return body, nil
	}

	var openAIResp map[string]interface{}
	if err := json.Unmarshal(body, &openAIResp); err != nil {
		return body, nil
	}

	// Ensure OpenAI-compatible format
	if _, ok := openAIResp["choices"]; !ok {
		if content, ok := openAIResp["content"]; ok {
			openAIResp = map[string]interface{}{
				"id":      openAIResp["id"],
				"model":   openAIResp["model"],
				"object":  "chat.completion",
				"choices": []map[string]interface{}{{"index": 0, "message": map[string]interface{}{"role": "assistant", "content": content}}},
				"usage":   openAIResp["usage"],
			}
		}
	}

	return json.Marshal(openAIResp)
}

func NormalizeHeaders(headers map[string]string) map[string]string {
	result := make(map[string]string)
	for k, v := range headers {
		lower := strings.ToLower(k)
		if strings.HasPrefix(lower, "x-") || lower == "content-type" || lower == "authorization" {
			result[k] = v
		}
	}
	return result
}
