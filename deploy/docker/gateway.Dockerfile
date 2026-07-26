FROM golang:1.22-alpine AS builder
WORKDIR /app
COPY aegis-gateway/go.mod aegis-gateway/go.sum ./aegis-gateway/
COPY aegis-gateway/ ./aegis-gateway/
RUN cd aegis-gateway && go build -o /bin/gateway ./cmd/gateway/

FROM alpine:3.20
RUN apk add --no-cache ca-certificates
COPY --from=builder /bin/gateway /usr/local/bin/aegis-gateway
EXPOSE 8000 8001
ENTRYPOINT ["aegis-gateway"]
