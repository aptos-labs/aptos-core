# Dockerfile for Nix-built aptos-node binary
# This version should work with properly built Nix packages

FROM ubuntu:24.04

# Install minimal runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy the Nix-built binary
COPY aptos-node /usr/local/bin/aptos-node

# Make binary executable
RUN chmod +x /usr/local/bin/aptos-node

# Set the binary as entrypoint
ENTRYPOINT ["/usr/local/bin/aptos-node"]