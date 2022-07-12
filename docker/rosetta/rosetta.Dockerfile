FROM debian:buster-20220228@sha256:fd510d85d7e0691ca551fe08e8a2516a86c7f24601a940a299b5fe5cdd22c03a AS debian-base

## get rust build environment ready
FROM rust:1.61-buster AS rust-base

WORKDIR /aptos
RUN apt-get update && apt-get install -y cmake curl clang git pkg-config libssl-dev libpq-dev

### Build Rust code ###
FROM rust-base as builder

ARG GIT_SHA
RUN git clone https://github.com/aptos-labs/aptos-core.git ./ && git reset $GIT_SHA --hard
RUN --mount=type=cache,target=/aptos/target --mount=type=cache,target=$CARGO_HOME/registry \
  cargo build --release \
  -p aptos-node \
  -p aptos-rosetta \
  && mkdir dist \
  && cp target/release/aptos-node dist/aptos-node \
  && cp target/release/aptos-rosetta dist/aptos-rosetta

### Create image with aptos-node and aptos-rosetta ###
FROM debian-base AS rosetta

RUN apt-get update && apt-get install -y libssl1.1 ca-certificates && apt-get clean && rm -r /var/lib/apt/lists/*

COPY --from=builder /aptos/dist/aptos-rosetta /usr/local/bin/aptos-rosetta

# Rosetta online API
EXPOSE 8082
# Rosetta offline API
EXPOSE 8083

# Capture backtrace on error
ENV RUST_BACKTRACE 1

WORKDIR /opt/aptos/data

ENTRYPOINT ["/usr/local/bin/aptos-rosetta"]
CMD ["online", "--config /opt/aptos/fullnode.yaml"]
