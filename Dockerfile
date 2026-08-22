# Build stage
FROM rust:1.97-alpine AS builder

# Install build dependencies
# `openssl-libs-static` matters: the musl target links statically, and without the static halves
# the build gets all the way to `ld` before failing on -lssl/-lcrypto.
RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static

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
FROM alpine:3.24

# Litestream supervises the server: `replicate -exec` runs both as one process tree, so the image
# needs no init system, and the server cannot be running while replication is not.
ARG LITESTREAM_VERSION=v0.3.13
# TARGETARCH comes from the builder; hardcoding an architecture produces an image whose
# supervisor cannot exec, and the failure looks like the server never starting.
ARG TARGETARCH
RUN apk add --no-cache ca-certificates wget && \
    wget -qO- "https://github.com/benbjohnson/litestream/releases/download/${LITESTREAM_VERSION}/litestream-${LITESTREAM_VERSION}-linux-${TARGETARCH}.tar.gz" \
      | tar -xz -C /usr/local/bin litestream && \
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

# Probes the liveness route, not `/`. They are the routes that are deliberately public and
# deliberately cheap; `/` is neither guaranteed.
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/healthz/live || exit 1

# Default environment variables
ENV SEALBOX_STORE_PATH=/data/sealbox.db
ENV SEALBOX_MASTER_KEY_PATH=/data/master.pem
ENV SEALBOX_LISTEN_ADDR=0.0.0.0:8080

# Litestream refuses to start without a config file, so the image carries one that names the
# database and replicates nowhere. That is right for a local run and wrong for production: mount
# your own over it, or point LITESTREAM_CONFIG at one, and check the logs for the replica being
# announced. A server whose database is not being replicated is the state that quietly costs
# everything later.
COPY <<'YAML' /etc/litestream.yml
dbs:
  - path: /data/sealbox.db
YAML

CMD ["litestream", "replicate", "-exec", "sealbox-server"]