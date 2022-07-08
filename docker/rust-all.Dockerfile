#syntax=docker/dockerfile:1.4

FROM debian:buster-20220228@sha256:fd510d85d7e0691ca551fe08e8a2516a86c7f24601a940a299b5fe5cdd22c03a AS debian-base

FROM rust:1.61-buster AS rust-base
WORKDIR /aptos
RUN apt-get update && apt-get install -y cmake curl clang git pkg-config libssl-dev libpq-dev

### Build Rust code ###
FROM rust-base as builder
COPY . /aptos/
RUN find .
RUN --mount=type=cache,target=/aptos/target --mount=type=cache,target=$CARGO_HOME/registry docker/build-rust-all.sh && rm -rf $CARGO_HOME/registry/index

### Validator Image ###
FROM debian-base AS validator

RUN apt-get update && apt-get install -y libssl1.1 ca-certificates && apt-get clean && rm -r /var/lib/apt/lists/*

### Needed to run debugging tools like perf
RUN apt-get update && apt-get install -y linux-perf sudo procps
### Because build machine perf might not match run machine perf, we have to symlink
### Even if version slightly off, still mostly works
RUN ln -sf /usr/bin/perf_* /usr/bin/perf

RUN addgroup --system --gid 6180 aptos && adduser --system --ingroup aptos --no-create-home --uid 6180 aptos

RUN mkdir -p /opt/aptos/bin /opt/aptos/etc
COPY --link --from=builder /aptos/target/release/aptos-node /opt/aptos/bin/
COPY --link --from=builder /aptos/target/release/db-backup /opt/aptos/bin/
COPY --link --from=builder /aptos/target/release/db-bootstrapper /opt/aptos/bin/
COPY --link --from=builder /aptos/target/release/db-restore /opt/aptos/bin/

# Admission control
EXPOSE 8000
# Validator network
EXPOSE 6180
# Metrics
EXPOSE 9101
# Backup
EXPOSE 6186

# Capture backtrace on error
ENV RUST_BACKTRACE 1



### Indexer Image ###

FROM debian-base AS indexer

RUN apt-get update && apt-get install -y libssl1.1 ca-certificates net-tools tcpdump iproute2 netcat libpq-dev \
    && apt-get clean && rm -r /var/lib/apt/lists/*

RUN mkdir -p /opt/aptos/bin
COPY --from=builder /aptos/target/release/aptos-indexer /usr/local/bin/

### Node Checker Image ###

FROM debian-base AS node-checker

RUN apt-get update && apt-get install -y libssl1.1 ca-certificates net-tools tcpdump iproute2 netcat libpq-dev \
    && apt-get clean && rm -r /var/lib/apt/lists/*

RUN mkdir -p /opt/aptos/bin
COPY --from=builder /aptos/target/release/aptos-node-checker /usr/local/bin/


### Tools Image ###
FROM debian-base AS tools

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

COPY --from=builder /aptos/target/release/aptos-genesis-tool /usr/local/bin/
COPY --from=builder /aptos/target/release/aptos-operational-tool /usr/local/bin/
COPY --from=builder /aptos/target/release/db-bootstrapper /usr/local/bin/
COPY --from=builder /aptos/target/release/db-backup /usr/local/bin/
COPY --from=builder /aptos/target/release/db-backup-verify /usr/local/bin/
COPY --from=builder /aptos/target/release/db-restore /usr/local/bin/
COPY --from=builder /aptos/target/release/aptos-transaction-replay /usr/local/bin/
COPY --from=builder /aptos/target/release/aptos /usr/local/bin/
COPY --from=builder /aptos/target/release/transaction-emitter /usr/local/bin/

### Get Aptos Move modules bytecodes for genesis ceremony
RUN mkdir -p /aptos-framework/move/build
RUN mkdir -p /aptos-framework/move/modules
COPY --from=builder /aptos/aptos-framework/releases/artifacts/current/build /aptos-framework/move/build
RUN mv /aptos-framework/move/build/**/bytecode_modules/*.mv /aptos-framework/move/modules
RUN mv /aptos-framework/move/build/**/bytecode_modules/dependencies/**/*.mv /aptos-framework/move/modules
RUN rm -rf /aptos-framework/move/build


### Faucet Image ###
FROM debian-base AS faucet

RUN apt-get update && apt-get install -y libssl1.1 ca-certificates nano net-tools tcpdump iproute2 netcat \
    && apt-get clean && rm -r /var/lib/apt/lists/*

RUN mkdir -p /opt/aptos/bin  /aptos/client/data/wallet/

COPY --from=builder /aptos/target/release/aptos-faucet /opt/aptos/bin/

#install needed tools
RUN apt-get update && apt-get install -y procps

# Mint proxy listening address
EXPOSE 8000



### Forge Image ###

FROM debian-base as forge

RUN apt-get update && apt-get install -y libssl1.1 ca-certificates openssh-client wget busybox git unzip awscli && apt-get clean && rm -r /var/lib/apt/lists/*

RUN mkdir /aptos

# copy helm charts from source
COPY --from=builder /aptos/terraform/helm /aptos/terraform/helm

RUN cd /usr/local/bin && wget "https://storage.googleapis.com/kubernetes-release/release/v1.18.6/bin/linux/amd64/kubectl" -O kubectl && chmod +x kubectl
RUN cd /usr/local/bin && wget "https://get.helm.sh/helm-v3.8.0-linux-amd64.tar.gz" -O- | busybox tar -zxvf - && mv linux-amd64/helm . && chmod +x helm
ENV PATH "$PATH:/root/bin"

WORKDIR /aptos
COPY --from=builder /aptos/target/release/forge /usr/local/bin/forge


ENTRYPOINT ["forge"]
