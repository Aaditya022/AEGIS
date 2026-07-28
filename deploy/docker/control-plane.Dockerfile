FROM golang:1.26-alpine AS builder
WORKDIR /app
COPY aegis-control-plane/go.mod aegis-control-plane/go.sum ./aegis-control-plane/
COPY aegis-control-plane/ ./aegis-control-plane/
RUN cd aegis-control-plane && go build -o /bin/controller ./cmd/controller/

FROM alpine:3.20
RUN apk add --no-cache ca-certificates
COPY --from=builder /bin/controller /usr/local/bin/aegis-control-plane
EXPOSE 8500 8501
ENTRYPOINT ["aegis-control-plane"]
