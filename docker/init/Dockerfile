FROM debian:buster-20220228@sha256:fd510d85d7e0691ca551fe08e8a2516a86c7f24601a940a299b5fe5cdd22c03a AS debian-base

FROM debian-base AS toolchain

# To use http/https proxy while building, use:
# docker build --build-arg https_proxy=http://fwdproxy:8080 --build-arg http_proxy=http://fwdproxy:8080

RUN apt-get update && apt-get install -y cmake curl clang git pkg-config libssl-dev libpq-dev

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain none
ENV PATH "$PATH:/root/.cargo/bin"

WORKDIR /aptos
COPY rust-toolchain /aptos/rust-toolchain
RUN rustup install $(cat rust-toolchain)

FROM toolchain AS builder

ARG ENABLE_FAILPOINTS
COPY . /aptos

RUN IMAGE_TARGET="release" ./docker/build-common.sh

### Production Image ###
FROM debian-base AS pre-prod

RUN apt-get update && apt-get -y install libssl1.1 ca-certificates wget busybox gettext-base && apt-get clean && rm -r /var/lib/apt/lists/*
RUN cd /usr/local/bin && wget "https://storage.googleapis.com/kubernetes-release/release/v1.18.6/bin/linux/amd64/kubectl" -O kubectl && chmod +x kubectl
RUN cd /usr/local/bin && wget "https://releases.hashicorp.com/vault/1.5.0/vault_1.5.0_linux_amd64.zip" -O- | busybox unzip - && chmod +x vault

RUN mkdir -p /opt/aptos/bin
COPY --from=builder /aptos/target/release/aptos-genesis-tool /usr/local/bin
COPY --from=builder /aptos/target/release/aptos-operational-tool /usr/local/bin

### Get Aptos Move modules bytecodes for genesis ceremony
RUN mkdir -p /aptos-framework/move/build
RUN mkdir -p /aptos-framework/move/modules
COPY --from=builder /aptos/aptos-framework/releases/artifacts/current/build /aptos-framework/move/build
RUN mv /aptos-framework/move/build/**/bytecode_modules/*.mv /aptos-framework/move/modules
RUN mv /aptos-framework/move/build/**/bytecode_modules/dependencies/**/*.mv /aptos-framework/move/modules
RUN rm -rf /aptos-framework/move/build

FROM pre-prod as testing

RUN /usr/local/bin/aptos-genesis-tool -h
RUN /usr/local/bin/aptos-operational-tool -h

FROM pre-prod as prod

ARG BUILD_DATE
ARG GIT_REV
ARG GIT_UPSTREAM

LABEL org.label-schema.schema-version="1.0"
LABEL org.label-schema.build-date=$BUILD_DATE
LABEL org.label-schema.vcs-ref=$GIT_REV
