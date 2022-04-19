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

RUN IMAGE_TARGETS="release" ./docker/build-common.sh

### Production Image ###
FROM debian-base AS prod

RUN echo "deb http://deb.debian.org/debian bullseye main" > /etc/apt/sources.list.d/bullseye.list && \
    echo "Package: *\nPin: release n=bullseye\nPin-Priority: 50" > /etc/apt/preferences.d/bullseye

RUN apt-get update && \
    apt-get --no-install-recommends --yes install wget curl libssl1.1 ca-certificates socat python3-botocore/bullseye awscli/bullseye && \
    apt-get clean && \
    rm -r /var/lib/apt/lists/*

RUN ln -s /usr/bin/python3 /usr/local/bin/python
COPY docker/tools/boto.cfg /etc

RUN cd /usr/local/bin && wget https://azcopyvnext.azureedge.net/release20210226/azcopy_linux_amd64_10.9.0.tar.gz -O- | tar --gzip --wildcards --extract '*/azcopy' --strip-components=1 --no-same-owner && chmod +x azcopy
RUN wget https://storage.googleapis.com/pub/gsutil.tar.gz -O- | tar --gzip --directory /opt --extract && ln -s /opt/gsutil/gsutil /usr/local/bin

COPY --from=builder /aptos/target/release/aptos-genesis-tool /usr/local/bin
COPY --from=builder /aptos/target/release/aptos-operational-tool /usr/local/bin
COPY --from=builder /aptos/target/release/db-bootstrapper /usr/local/bin
COPY --from=builder /aptos/target/release/db-backup /usr/local/bin
COPY --from=builder /aptos/target/release/db-backup-verify /usr/local/bin
COPY --from=builder /aptos/target/release/db-restore /usr/local/bin
COPY --from=builder /aptos/target/release/aptos-transaction-replay /usr/local/bin

### Get Aptos Move modules bytecodes for genesis ceremony
RUN mkdir -p /aptos-framework/move/build
RUN mkdir -p /aptos-framework/move/modules
COPY --from=builder /aptos/aptos-framework/releases/artifacts/current/build /aptos-framework/move/build
RUN mv /aptos-framework/move/build/**/bytecode_modules/*.mv /aptos-framework/move/modules
RUN rm -rf /aptos-framework/move/build

ARG BUILD_DATE
ARG GIT_REV
ARG GIT_UPSTREAM

LABEL org.label-schema.schema-version="1.0"
LABEL org.label-schema.build-date=$BUILD_DATE
LABEL org.label-schema.vcs-ref=$GIT_REV
