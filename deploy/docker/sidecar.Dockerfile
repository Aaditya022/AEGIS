FROM rust:1.79-slim-bookworm AS builder
RUN apt-get update && apt-get install -y protobuf-compiler libclang-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY aegis-common/ aegis-common/
COPY aegis-sidecar/ aegis-sidecar/
COPY aegis-policy-engine/ aegis-policy-engine/
COPY proto/ proto/
RUN cargo build --release -p aegis-sidecar

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates iptables && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/aegis-sidecar /usr/local/bin/aegis-sidecar
COPY policies/ /etc/aegis/policies/
EXPOSE 9000 9001
ENTRYPOINT ["aegis-sidecar"]
CMD ["--config", "/etc/aegis/sidecar.yaml"]
