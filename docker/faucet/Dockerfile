FROM debian:buster-20220228@sha256:fd510d85d7e0691ca551fe08e8a2516a86c7f24601a940a299b5fe5cdd22c03a AS debian-base

FROM debian-base AS toolchain

# To use http/https proxy while building, use:
# docker build --build-arg https_proxy=http://fwdproxy:8080 --build-arg http_proxy=http://fwdproxy:8080

RUN apt-get update && apt-get install -y cmake curl clang git pkg-config libssl-dev

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain none
ENV PATH "$PATH:/root/.cargo/bin"

WORKDIR /aptos
COPY rust-toolchain /aptos/rust-toolchain
RUN rustup install $(cat rust-toolchain)

FROM toolchain AS builder

ARG ENABLE_FAILPOINTS
COPY . /aptos

RUN IMAGE_TARGETS="test" ./docker/build-common.sh

### Production Image ###
FROM debian-base AS pre-test

RUN apt-get update && apt-get install -y libssl1.1 ca-certificates nano net-tools tcpdump iproute2 netcat \
    && apt-get clean && rm -r /var/lib/apt/lists/*

RUN mkdir -p /opt/aptos/bin  /aptos/client/data/wallet/

COPY --from=builder /aptos/target/release/aptos-faucet /opt/aptos/bin

# Test the docker container before shipping.
FROM pre-test AS test

#install needed tools
RUN apt-get update && apt-get install -y procps

FROM pre-test as prod
# Mint proxy listening address
EXPOSE 8000

ARG BUILD_DATE
ARG GIT_REV
ARG GIT_UPSTREAM

LABEL org.label-schema.schema-version="1.0"
LABEL org.label-schema.build-date=$BUILD_DATE
LABEL org.label-schema.vcs-ref=$GIT_REV
