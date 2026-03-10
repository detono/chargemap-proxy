# Stage 1: Builder
FROM rust:1.85 AS builder
WORKDIR /app
ENV SQLX_OFFLINE=true

# Standard build dependencies
RUN apt-get update && apt-get install -y pkg-config && rm -rf /var/lib/apt/lists/*

COPY . .

# We just build normally. Docker Buildx will automatically run this 
# in the correct architecture (Native on Mac, Emulated for Intel).
RUN cargo build --release

# Stage 2: Runtime
FROM ubuntu:24.04

LABEL org.opencontainers.image.source="https://github.com/detono/chargemap-proxy"
LABEL org.opencontainers.image.description="OpenChargeMap Proxy API"

WORKDIR /app

# Install standard certificates and curl
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*

# Copy the binary from the standard release folder
COPY --from=builder /app/target/release/chargemap-proxy ./chargemap-proxy
COPY config.toml .

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8082/health || exit 1

COPY entrypoint.sh .
RUN chmod +x entrypoint.sh
ENTRYPOINT ["./entrypoint.sh"]

EXPOSE 8082
CMD ["./chargemap-proxy"]
