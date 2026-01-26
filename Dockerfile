# Build stage
FROM rust:1.93-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev pkgconfig openssl-dev

# Set working directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY sealbox-server/Cargo.toml ./sealbox-server/
COPY sealbox-cli/Cargo.toml ./sealbox-cli/

# Create dummy source files to cache dependencies
RUN mkdir -p sealbox-server/src sealbox-cli/src && \
    echo "fn main() {}" > sealbox-server/src/main.rs && \
    echo "fn main() {}" > sealbox-cli/src/main.rs && \
    echo "pub fn lib() {}" > sealbox-server/src/lib.rs

# Build dependencies (this will be cached)
RUN cargo build --release && \
    rm -rf sealbox-*/src

# Copy actual source code
COPY sealbox-server/src ./sealbox-server/src
COPY sealbox-cli/src ./sealbox-cli/src

# Build for release (only rebuild our code)
RUN touch sealbox-server/src/main.rs sealbox-server/src/lib.rs sealbox-cli/src/main.rs && \
    cargo build --release

# Runtime stage
FROM alpine:3.23

# Install runtime dependencies
RUN apk add --no-cache ca-certificates && \
    adduser -D -s /bin/sh sealbox

# Copy binaries from builder stage
COPY --from=builder /app/target/release/sealbox-server /usr/local/bin/
COPY --from=builder /app/target/release/sealbox-cli /usr/local/bin/

# Create data directory
RUN mkdir -p /data && chown sealbox:sealbox /data

# Switch to non-root user
USER sealbox

# Set working directory
WORKDIR /data

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/ || exit 1

# Default environment variables
ENV STORE_PATH=/data/sealbox.db
ENV LISTEN_ADDR=0.0.0.0:8080

# Default command
CMD ["sealbox-server"]