# Build context is relative to aptos-core repo root.

# Build CLI from source.
FROM rust:1-bookworm AS compile-cli
COPY . /app/
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
    --manifest-path app/Cargo.toml \
    --package aptos \
    --profile cli

# Copy over compiled binary and install dependencies.
FROM debian:bookworm-slim AS run-cli
COPY --from=compile-cli /app/target/cli/aptos /usr/local/bin
RUN apt-get update && apt-get install -y \
    curl \
    libssl-dev \
    python3 \
    && rm -rf /var/lib/apt/lists/*
RUN curl -sSL https://install.python-poetry.org | python3 -
ENV PATH="/root/.local/bin:$PATH"
COPY aptos-move/move-examples/lockstream/pyproject.toml /app/
RUN poetry install --directory /app/

# Initialize chain state with funded accounts, mint authority.
FROM run-cli AS init-chain-state
COPY aptos-move/move-examples/lockstream/accounts /accounts/
COPY . /aptos-core/
RUN aptos node run-local-testnet \
        --test-dir /app/data \
        --with-faucet \
        & sleep 30 && \
    aptos init --network local --private-key-file accounts/ace.key --profile ace && \
    aptos init --network local --private-key-file accounts/bee.key --profile bee && \
    aptos init --network local --private-key-file accounts/cad.key --profile cad && \
    aptos init --network local --private-key-file accounts/dee.key --profile dee && \
    aptos move publish \
        --assume-yes \
        --included-artifacts none \
        --named-addresses dee=$(cat accounts/dee.address) \
        --override-size-check \
        --package-dir /aptos-core/aptos-move/move-examples/lockstream/assets \
        --profile dee

# Copy over chain state and accounts.
FROM run-cli
COPY --from=init-chain-state /app/data /app/data
COPY --from=init-chain-state /accounts /app/accounts

# Serve local testnet, run example script.
WORKDIR /app/
ENTRYPOINT [ \
    "/usr/local/bin/aptos", "node", "run-local-testnet", \
    "--test-dir", "/app/data", \
    "--with-faucet" \
    "&&", \
    "/root/.local/bin/poetry", "run", "python", \
    "script/example.py" \
]