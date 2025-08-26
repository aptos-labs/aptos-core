# Use Ubuntu 24.04 for newer glibc compatibility
FROM ubuntu:24.04

# Install required packages and create symlink for librocksdb
RUN apt-get update && apt-get install -y \
    libjemalloc2 \
    libdw1 \
    librocksdb-dev \
    libssl3 \
    libstdc++6 \
    zlib1g \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && ln -sf /usr/lib/x86_64-linux-gnu/librocksdb.so /usr/lib/x86_64-linux-gnu/librocksdb.so.10

# Create app directory
WORKDIR /app

# Copy the pre-built binary (now in build context)
COPY aptos-node /usr/local/bin/aptos-node

# Make binary executable
RUN chmod +x /usr/local/bin/aptos-node

# Expose default ports
EXPOSE 8080 9101 6180

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD aptos-node --help || exit 1

# Run aptos-node by default
ENTRYPOINT ["aptos-node"]
CMD ["--help"]