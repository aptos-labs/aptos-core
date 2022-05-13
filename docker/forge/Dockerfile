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

FROM debian-base

RUN apt-get update && apt-get install -y libssl1.1 ca-certificates openssh-client wget busybox git unzip awscli && apt-get clean && rm -r /var/lib/apt/lists/*

RUN mkdir /aptos
COPY rust-toolchain /aptos/rust-toolchain

RUN cd /usr/local/bin && wget "https://storage.googleapis.com/kubernetes-release/release/v1.18.6/bin/linux/amd64/kubectl" -O kubectl && chmod +x kubectl
RUN cd /usr/local/bin && wget "https://releases.hashicorp.com/vault/1.5.0/vault_1.5.0_linux_amd64.zip" -O- | busybox unzip - && chmod +x vault
RUN cd /usr/local/bin && wget "https://get.helm.sh/helm-v3.8.0-linux-amd64.tar.gz" -O- | busybox tar -zxvf - && mv linux-amd64/helm . && chmod +x helm
ENV PATH "$PATH:/root/bin"

RUN helm plugin install https://github.com/hypnoglow/helm-s3.git --version 0.10.0

RUN mkdir /etc/forge
WORKDIR /etc/forge
COPY --from=builder /aptos/target/release/forge /usr/local/bin/forge
ENTRYPOINT ["forge"]
ARG BUILD_DATE
ARG GIT_REV
ARG GIT_UPSTREAM
LABEL org.label-schema.schema-version="1.0"
LABEL org.label-schema.build-date=$BUILD_DATE
LABEL org.label-schema.vcs-ref=$GIT_REV
