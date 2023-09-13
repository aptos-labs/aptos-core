# Build context is relative to aptos-core repo root.

# Relative to image root.
ARG APP_DIR=/app

# Relative to image root, after copying in aptos-core to /app/
ARG EXAMPLE_DIR=/app/aptos-move/move-examples/lockstream
ARG IMAGE_RESOURCES=$EXAMPLE_DIR/image-resources
ARG ACCOUNTS=$IMAGE_RESOURCES/accounts
ARG ASSETS_PACKAGE=$IMAGE_RESOURCES/assets

# Build CLI from source.
FROM rust:1-bookworm AS compile-cli
ARG APP_DIR
COPY . $APP_DIR
RUN apt-get update && apt-get install -y \
    build-essential \
    libclang-dev \
    libpq-dev \
    libssl-dev \
    libudev-dev \
    lld \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*
RUN RUSTFLAGS="--cfg tokio_unstable" cargo build \
    --bin aptos \
    --manifest-path $APP_DIR/Cargo.toml \
    --package aptos \
    --profile cli

# Copy over compiled binary and install dependencies.
FROM debian:bookworm-slim AS run-cli
ARG APP_DIR
COPY --from=compile-cli /$APP_DIR/target/cli/aptos /usr/local/bin
RUN apt-get update && apt-get install -y \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Initialize chain state with funded accounts, mint authority.
FROM run-cli AS init-chain-state
ARG ACCOUNTS
ARG APP_DIR
ARG ASSETS_PACKAGE
COPY . $APP_DIR
RUN aptos node run-local-testnet \
        --test-dir $APP_DIR/data \
        --with-faucet \
        & sleep 30 && \
    aptos init --network local --private-key-file $ACCOUNTS/ace.key --profile ace && \
    aptos init --network local --private-key-file $ACCOUNTS/bee.key --profile bee && \
    aptos init --network local --private-key-file $ACCOUNTS/cad.key --profile cad && \
    aptos init --network local --private-key-file $ACCOUNTS/dee.key --profile dee && \
    aptos move publish \
        --assume-yes \
        --included-artifacts none \
        --named-addresses dee=$(cat $ACCOUNTS/dee.address) \
        --override-size-check \
        --package-dir $ASSETS_PACKAGE \
        --profile dee

# Copy over chain state, accounts and profile info.
FROM run-cli
COPY --from=init-chain-state /app/data /app/data
COPY --from=init-chain-state $ACCOUNTS /app/accounts
COPY --from=init-chain-state /.aptos /.aptos