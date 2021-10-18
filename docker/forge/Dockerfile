FROM debian:buster-20210927@sha256:37b68406b219ca10fd50a49340da9e6e9ac189deecf75c3d3bd90a016d29fbb0 AS debian-base

FROM debian-base AS toolchain

# To use http/https proxy while building, use:
# docker build --build-arg https_proxy=http://fwdproxy:8080 --build-arg http_proxy=http://fwdproxy:8080

RUN apt-get update && apt-get install -y cmake curl clang git pkg-config libssl-dev

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain none
ENV PATH "$PATH:/root/.cargo/bin"

WORKDIR /diem
COPY rust-toolchain /diem/rust-toolchain
RUN rustup install $(cat rust-toolchain)

FROM toolchain AS builder

ARG ENABLE_FAILPOINTS
COPY . /diem

RUN IMAGE_TARGETS="test" ./docker/build-common.sh

FROM debian-base

RUN apt-get update && apt-get install -y libssl1.1 openssh-client wget && apt-get clean && rm -r /var/lib/apt/lists/*

RUN mkdir /diem
COPY rust-toolchain /diem/rust-toolchain
COPY scripts/dev_setup.sh /diem/scripts/dev_setup.sh
RUN /diem/scripts/dev_setup.sh -b -p -i kubectl -i helm -i git -i unzip -i awscli -i vault
ENV PATH "$PATH:/root/bin"

RUN helm plugin install https://github.com/hypnoglow/helm-s3.git --version 0.10.0

RUN mkdir /etc/forge
WORKDIR /etc/forge
COPY --from=builder /diem/target/release/forge /usr/local/bin/forge
ENTRYPOINT ["forge"]
ARG BUILD_DATE
ARG GIT_REV
ARG GIT_UPSTREAM
LABEL org.label-schema.schema-version="1.0"
LABEL org.label-schema.build-date=$BUILD_DATE
LABEL org.label-schema.vcs-ref=$GIT_REV
