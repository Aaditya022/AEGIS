module github.com/aegis-ai/aegis/control-plane

go 1.22

require (
	github.com/go-redis/redis/v8 v8.11.5
	github.com/gorilla/mux v1.8.1
	github.com/lib/pq v1.10.9
	github.com/prometheus/client_golang v1.19.0
	go.etcd.io/etcd/client/v3 v3.5.14
	go.opentelemetry.io/otel v1.27.0
	go.opentelemetry.io/otel/exporters/otlp/otlptrace v1.27.0
	go.opentelemetry.io/otel/sdk v1.27.0
	go.uber.org/zap v1.27.0
	google.golang.org/grpc v1.64.0
	google.golang.org/protobuf v1.34.1
)
