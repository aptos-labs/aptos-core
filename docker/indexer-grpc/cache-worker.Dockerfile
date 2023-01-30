FROM ubuntu:20.04@sha256:a06ae92523384c2cd182dcfe7f8b2bf09075062e937d5653d7d0db0375ad2221 AS ubuntu-base

## get rust build environment ready
FROM rust:1.63.0-buster@sha256:0110d1b4193029735f1db1c0ed661676ed4b6f705b11b1ebe95c655b52e6906f AS rust-base

WORKDIR /aptos
RUN apt-get update && apt-get install -y cmake curl clang git pkg-config libssl-dev libpq-dev lld

### Build Rust code ###
FROM rust-base as builder

ARG GIT_REPO=https://github.com/aptos-labs/aptos-core.git
ARG GIT_REF

RUN git clone $GIT_REPO ./ && git fetch origin $GIT_BRANCH && git checkout $GIT_BRANCH && git reset $GIT_REF --hard
# Compile aptos-indexer-grpc-cache-worker
RUN --mount=type=cache,target=/aptos/target --mount=type=cache,target=$CARGO_HOME/registry \
  cargo build --release \
  -p aptos-indexer-grpc-cache-worker \
  && mkdir dist \
  && cp target/release/aptos-indexer-grpc-cache-worker dist/aptos-indexer-grpc-cache-worker

# Compile aptos-indexer-grpc-file-store
RUN --mount=type=cache,target=/aptos/target --mount=type=cache,target=$CARGO_HOME/registry \
  cargo build --release \
  -p aptos-indexer-grpc-file-store \
  && cp target/release/aptos-indexer-grpc-file-store dist/aptos-indexer-grpc-file-store

# Compile aptos-indexer-grpc-data-service
RUN --mount=type=cache,target=/aptos/target --mount=type=cache,target=$CARGO_HOME/registry \
  cargo build --release \
  -p aptos-indexer-grpc-data-service \
  && cp target/release/aptos-indexer-grpc-data-service dist/aptos-indexer-grpc-data-service

FROM ubuntu-base AS indexer-grpc

RUN apt-get update && apt-get install -y libssl-dev ca-certificates && apt-get clean && rm -r /var/lib/apt/lists/*

COPY --from=builder /aptos/dist/aptos-indexer-grpc-cache-worker /usr/local/bin/aptos-indexer-grpc-cache-worker
COPY --from=builder /aptos/dist/aptos-indexer-grpc-file-store /usr/local/bin/aptos-indexer-grpc-file-store
COPY --from=builder /aptos/dist/aptos-indexer-grpc-data-service /usr/local/bin/aptos-indexer-grpc-data-service

# Health check
EXPOSE 8080
# GRPC
EXPOSE 50051
# Capture backtrace on error
ENV RUST_BACKTRACE 1

WORKDIR /opt/aptos/data

CMD ["/usr/local/bin/aptos-indexer-grpc-cache-worker"]
