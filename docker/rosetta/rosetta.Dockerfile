FROM ubuntu:20.04@sha256:a06ae92523384c2cd182dcfe7f8b2bf09075062e937d5653d7d0db0375ad2221 AS ubuntu-base

## get rust build environment ready
FROM rust:1.63.0-buster@sha256:0110d1b4193029735f1db1c0ed661676ed4b6f705b11b1ebe95c655b52e6906f AS rust-base

WORKDIR /aptos
RUN apt-get update && apt-get install -y cmake curl clang git pkg-config libssl-dev libpq-dev

### Build Rust code ###
FROM rust-base as builder

ARG GIT_REPO=https://github.com/aptos-labs/aptos-core.git
ARG GIT_REF

RUN git clone $GIT_REPO ./ && git reset $GIT_REF --hard
RUN --mount=type=cache,target=/aptos/target --mount=type=cache,target=$CARGO_HOME/registry \
  cargo build --release \
  -p aptos-rosetta \
  && mkdir dist \
  && cp target/release/aptos-rosetta dist/aptos-rosetta

### Create image with aptos-node and aptos-rosetta ###
FROM ubuntu-base AS rosetta

RUN apt-get update && apt-get install -y libssl-dev ca-certificates && apt-get clean && rm -r /var/lib/apt/lists/*

COPY --from=builder /aptos/dist/aptos-rosetta /usr/local/bin/aptos-rosetta

# Rosetta API
EXPOSE 8082

# Capture backtrace on error
ENV RUST_BACKTRACE 1

WORKDIR /opt/aptos/data

ENTRYPOINT ["/usr/local/bin/aptos-rosetta"]
CMD ["online", "--config /opt/aptos/fullnode.yaml"]
