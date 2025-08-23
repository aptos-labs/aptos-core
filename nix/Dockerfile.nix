# Dockerfile for Aptos Core using Nix
FROM nixos/nix:latest

# Install git to clone the repository
RUN nix-env -iA nixpkgs.git

# Create working directory
WORKDIR /app

# Clone the aptos-core repository
RUN git clone https://github.com/aptos-labs/aptos-core.git .
# Note: In practice, you would copy the local files instead of cloning

# Copy the nix directory
COPY nix/ ./nix/

# Build aptos-node using Nix
RUN nix build .#aptos-core

# Create a minimal image with just the binary
FROM alpine:latest

# Install ca-certificates for HTTPS requests
RUN apk --no-cache add ca-certificates

# Create app directory
WORKDIR /app

# Copy the binary from the build stage
COPY --from=0 /app/result/bin/aptos-node /usr/local/bin/aptos-node

# Expose default ports
EXPOSE 8080 9101 6180

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD aptos-node --help || exit 1

# Run aptos-node by default
ENTRYPOINT ["aptos-node"]
CMD ["--help"]