package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"sync"
	"syscall"
	"time"

	"go.etcd.io/etcd/client/v3"
	"go.uber.org/zap"
)

type AgentRecord struct {
	AgentID    string            `json:"agent_id"`
	Framework  string            `json:"framework"`
	Status     string            `json:"status"`
	Labels     map[string]string `json:"labels"`
	Registered time.Time         `json:"registered_at"`
	LastSeen   time.Time         `json:"last_seen"`
}

type PolicyRecord struct {
	ID       string `json:"id"`
	Name     string `json:"name"`
	Category string `json:"category"`
	Severity string `json:"severity"`
	Source   string `json:"source"`
	Enabled  bool   `json:"enabled"`
}

type ControlPlane struct {
	mu       sync.RWMutex
	agents   map[string]*AgentRecord
	policies map[string]*PolicyRecord
	logger   *zap.Logger
	etcd     *clientv3.Client
}

func NewControlPlane(logger *zap.Logger, etcd *clientv3.Client) *ControlPlane {
	cp := &ControlPlane{
		agents:   make(map[string]*AgentRecord),
		policies: make(map[string]*PolicyRecord),
		logger:   logger,
		etcd:     etcd,
	}
	if etcd != nil {
		cp.loadFromEtcd()
	}
	return cp
}

func (cp *ControlPlane) loadFromEtcd() {
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	resp, err := cp.etcd.Get(ctx, "/aegis/agents/", clientv3.WithPrefix())
	if err != nil {
		cp.logger.Warn("Failed to load agents from etcd", zap.Error(err))
	} else {
		for _, kv := range resp.Kvs {
			var agent AgentRecord
			if err := json.Unmarshal(kv.Value, &agent); err == nil {
				cp.agents[agent.AgentID] = &agent
			}
		}
		cp.logger.Info("Loaded agents from etcd", zap.Int("count", len(cp.agents)))
	}

	resp, err = cp.etcd.Get(ctx, "/aegis/policies/", clientv3.WithPrefix())
	if err != nil {
		cp.logger.Warn("Failed to load policies from etcd", zap.Error(err))
	} else {
		for _, kv := range resp.Kvs {
			var policy PolicyRecord
			if err := json.Unmarshal(kv.Value, &policy); err == nil {
				cp.policies[policy.ID] = &policy
			}
		}
		cp.logger.Info("Loaded policies from etcd", zap.Int("count", len(cp.policies)))
	}
}

func (cp *ControlPlane) saveAgentToEtcd(agent *AgentRecord) {
	if cp.etcd == nil {
		return
	}
	data, err := json.Marshal(agent)
	if err != nil {
		cp.logger.Warn("Failed to marshal agent", zap.Error(err))
		return
	}
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()
	key := fmt.Sprintf("/aegis/agents/%s", agent.AgentID)
	if _, err := cp.etcd.Put(ctx, key, string(data)); err != nil {
		cp.logger.Warn("Failed to save agent to etcd", zap.Error(err))
	}
}

func (cp *ControlPlane) savePolicyToEtcd(policy *PolicyRecord) {
	if cp.etcd == nil {
		return
	}
	data, err := json.Marshal(policy)
	if err != nil {
		cp.logger.Warn("Failed to marshal policy", zap.Error(err))
		return
	}
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()
	key := fmt.Sprintf("/aegis/policies/%s", policy.ID)
	if _, err := cp.etcd.Put(ctx, key, string(data)); err != nil {
		cp.logger.Warn("Failed to save policy to etcd", zap.Error(err))
	}
}

func (cp *ControlPlane) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")

	switch {
	case r.Method == http.MethodGet && r.URL.Path == "/v1/health":
		status := map[string]string{"status": "ok", "service": "aegis-control-plane"}
		if cp.etcd != nil {
			status["backend"] = "etcd"
		} else {
			status["backend"] = "in-memory"
		}
		json.NewEncoder(w).Encode(status)

	case r.Method == http.MethodGet && r.URL.Path == "/v1/agents":
		cp.mu.RLock()
		agents := make([]*AgentRecord, 0, len(cp.agents))
		for _, a := range cp.agents {
			agents = append(agents, a)
		}
		cp.mu.RUnlock()
		json.NewEncoder(w).Encode(agents)

	case r.Method == http.MethodPost && r.URL.Path == "/v1/agents":
		var agent AgentRecord
		if err := json.NewDecoder(r.Body).Decode(&agent); err != nil {
			http.Error(w, fmt.Sprintf(`{"error":%q}`, err.Error()), http.StatusBadRequest)
			return
		}
		cp.mu.Lock()
		agent.Registered = time.Now()
		agent.LastSeen = time.Now()
		cp.agents[agent.AgentID] = &agent
		cp.mu.Unlock()
		cp.saveAgentToEtcd(&agent)
		cp.logger.Info("Agent registered", zap.String("agent_id", agent.AgentID), zap.String("framework", agent.Framework))
		w.WriteHeader(http.StatusCreated)
		json.NewEncoder(w).Encode(agent)

	case r.Method == http.MethodGet && len(r.URL.Path) > 10 && r.URL.Path[:10] == "/v1/agents/":
		id := r.URL.Path[10:]
		cp.mu.RLock()
		agent, ok := cp.agents[id]
		cp.mu.RUnlock()
		if !ok {
			http.Error(w, `{"error":"agent not found"}`, http.StatusNotFound)
			return
		}
		json.NewEncoder(w).Encode(agent)

	case r.Method == http.MethodGet && r.URL.Path == "/v1/policies":
		cp.mu.RLock()
		policies := make([]*PolicyRecord, 0, len(cp.policies))
		for _, p := range cp.policies {
			policies = append(policies, p)
		}
		cp.mu.RUnlock()
		json.NewEncoder(w).Encode(policies)

	case r.Method == http.MethodPost && r.URL.Path == "/v1/policies":
		var policy PolicyRecord
		if err := json.NewDecoder(r.Body).Decode(&policy); err != nil {
			http.Error(w, fmt.Sprintf(`{"error":%q}`, err.Error()), http.StatusBadRequest)
			return
		}
		cp.mu.Lock()
		cp.policies[policy.ID] = &policy
		cp.mu.Unlock()
		cp.savePolicyToEtcd(&policy)
		cp.logger.Info("Policy pushed", zap.String("policy_id", policy.ID), zap.String("category", policy.Category))
		w.WriteHeader(http.StatusCreated)
		json.NewEncoder(w).Encode(policy)

	case r.Method == http.MethodGet && r.URL.Path == "/v1/inspect":
		cp.mu.RLock()
		state := map[string]interface{}{
			"agents":   cp.agents,
			"policies": cp.policies,
		}
		cp.mu.RUnlock()
		json.NewEncoder(w).Encode(state)

	default:
		http.Error(w, `{"error":"not found"}`, http.StatusNotFound)
	}
}

func main() {
	logger, _ := zap.NewProduction()
	defer logger.Sync()

	port := os.Getenv("AEGIS_CONTROL_PLANE_PORT")
	if port == "" {
		port = "8500"
	}

	// Connect to etcd for persistence
	etcdEndpoints := os.Getenv("AEGIS_ETCD_ENDPOINTS")
	var etcdClient *clientv3.Client
	if etcdEndpoints != "" {
		var err error
		etcdClient, err = clientv3.New(clientv3.Config{
			Endpoints:   []string{etcdEndpoints},
			DialTimeout: 5 * time.Second,
		})
		if err != nil {
			logger.Warn("Failed to connect to etcd, using in-memory backend", zap.Error(err))
			etcdClient = nil
		} else {
			logger.Info("Connected to etcd", zap.String("endpoints", etcdEndpoints))
		}
	} else {
		logger.Warn("AEGIS_ETCD_ENDPOINTS not set, using in-memory backend")
	}

	cp := NewControlPlane(logger, etcdClient)
	logger.Info("Starting AEGIS Control Plane", zap.String("port", port))

	server := &http.Server{
		Addr:    fmt.Sprintf(":%s", port),
		Handler: cp,
	}

	go func() {
		if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			logger.Fatal("Server error", zap.Error(err))
		}
	}()

	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
	<-quit

	logger.Info("Shutting down control plane...")
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	server.Shutdown(ctx)
	if etcdClient != nil {
		etcdClient.Close()
	}
	logger.Info("Control plane stopped")
}
