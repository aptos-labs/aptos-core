# Buld context is relative to aptos-core root
FROM rust:1-bookworm AS compile-cli
COPY . /app/
#RUN apt-get update && apt-get install -y \
    #build-essential \
    #libclang-dev \
    #libpq-dev \
    #libssl-dev \
    #lld \
    #&& rm -rf /var/lib/apt/lists/*
#RUN RUSTFLAGS="--cfg tokio_unstable" cargo build \
    #--bin aptos \
    #--manifest-path app/Cargo.toml \
    #--package aptos \
    #--profile cli

FROM debian:bookworm-slim AS runtime
#COPY --from=compile-cli /app/target/cli/aptos /usr/local/bin
RUN apt-get update && apt-get install -y \
    curl \
    libssl-dev \
    python3 \
    && rm -rf /var/lib/apt/lists/*
RUN curl -sSL https://install.python-poetry.org | python3 -
ENV PATH="/root/.local/bin:$PATH"
COPY aptos-move/move-examples/lockstream/pyproject.toml /app/
RUN poetry install --directory /app/

WORKDIR /app/
ENTRYPOINT [ \
    "/root/.local/bin/poetry", \
    "run", \
    "python", \
    "script/example.py" \
]