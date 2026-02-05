# Multi-platform Dockerfile for aptos-node using Nix
# Builds inside Docker using Nix, works on macOS/Apple Silicon and Linux
#
# Usage:
#   docker buildx build --platform linux/amd64 -f docker/aptos-node/Dockerfile.nix \
#     -t ghcr.io/movementlabsxyz/aptos-node:nix .
#   OR use: just container-buildx aptos-node
#
# The build will be skipped if the binaries are already cached in the Nix store

# Stage 1: Build using Nix inside a Linux container
FROM nixos/nix:2.24.10 AS builder

# Enable flakes and configure Nix for efficient builds
RUN mkdir -p /etc/nix && \
    echo "experimental-features = nix-command flakes" >> /etc/nix/nix.conf && \
    echo "sandbox = false" >> /etc/nix/nix.conf && \
    echo "filter-syscalls = false" >> /etc/nix/nix.conf

WORKDIR /build

# Copy only files needed for Nix evaluation first (better layer caching)
COPY nix/ ./nix/
COPY rust-toolchain.toml ./
COPY Cargo.toml Cargo.lock ./

# Copy the full source code
COPY . .

# Build all binaries using Nix
# The --no-link avoids creating symlinks, we use --print-out-paths to get the path
RUN cd nix && \
    nix build .#all-binaries -L && \
    mkdir -p /output/bin && \
    cp -L result/bin/* /output/bin/ && \
    chmod +x /output/bin/*

# Stage 2: Minimal runtime image
FROM docker.io/library/ubuntu:24.04 AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    libjemalloc2 \
    libdw1 \
    librocksdb-dev \
    libssl3 \
    libstdc++6 \
    zlib1g \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && ln -sf /usr/lib/x86_64-linux-gnu/librocksdb.so /usr/lib/x86_64-linux-gnu/librocksdb.so.10 || true

# Copy binaries from builder stage
COPY --from=builder /output/bin/aptos-node /usr/local/bin/
COPY --from=builder /output/bin/movement /usr/local/bin/
COPY --from=builder /output/bin/l1-migration /usr/local/bin/

# Create backwards compatibility symlink (aptos -> movement)
RUN ln -sf /usr/local/bin/movement /usr/local/bin/aptos

# Set working directory
WORKDIR /app

# Build metadata
ARG BUILD_DATE
ARG GIT_TAG
ARG GIT_BRANCH
ARG GIT_SHA
ENV BUILD_DATE=${BUILD_DATE} \
    GIT_TAG=${GIT_TAG} \
    GIT_BRANCH=${GIT_BRANCH} \
    GIT_SHA=${GIT_SHA}

# Verify binary is executable
RUN /usr/local/bin/aptos-node --version || echo "Note: Version check may fail during build"

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD curl -sf http://localhost:8080/v1 || exit 1

# Expose ports
# 8080 - REST API
# 6180 - Validator network
# 6181 - Full node network
# 9101 - Metrics
EXPOSE 8080 6180 6181 9101

# Default entrypoint
ENTRYPOINT ["/usr/local/bin/aptos-node"]
CMD ["--version"]