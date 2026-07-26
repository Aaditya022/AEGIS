module github.com/aegis-ai/aegis/gateway

go 1.22

require (
	github.com/cespare/xxhash/v2 v2.3.0
	github.com/go-redis/redis/v8 v8.11.5
	github.com/hashicorp/golang-lru/v2 v2.0.7
	github.com/prometheus/client_golang v1.19.0
	github.com/rs/zerolog v1.33.0
	go.opentelemetry.io/otel v1.27.0
	go.opentelemetry.io/otel/exporters/otlp/otlptrace/otlptracegrpc v1.27.0
	go.opentelemetry.io/otel/sdk v1.27.0
	go.opentelemetry.io/otel/trace v1.27.0
	google.golang.org/grpc v1.64.0
	google.golang.org/protobuf v1.34.1
	gopkg.in/yaml.v3 v3.0.1
)
