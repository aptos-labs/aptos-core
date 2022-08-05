#syntax=docker/dockerfile:1.4

FROM debian:buster-20220228@sha256:fd510d85d7e0691ca551fe08e8a2516a86c7f24601a940a299b5fe5cdd22c03a AS debian-base

# Add Tini to make sure the binaries receive proper SIGTERM signals when Docker is shut down
ADD https://github.com/krallin/tini/releases/download/v0.19.0/tini /tini
RUN chmod +x /tini
ENTRYPOINT ["/tini", "--"]

FROM rust:1.61-buster AS rust-base
WORKDIR /aptos
RUN apt-get update && apt-get install -y cmake curl clang git pkg-config libssl-dev libpq-dev

### Build Rust code ###
FROM rust-base as builder

# Confirm that this Dockerfile is being invoked from an appropriate builder.
# See https://github.com/aptos-labs/aptos-core/pull/2471
# See https://github.com/aptos-labs/aptos-core/pull/2472
ARG BUILT_VIA_BUILDKIT
ENV BUILT_VIA_BUILDKIT $BUILT_VIA_BUILDKIT

RUN test -n "$BUILT_VIA_BUILDKIT" || (printf "===\nREAD ME\n===\n\nYou likely just tried run a docker build using this Dockerfile using\nthe standard docker builder (e.g. docker build). The standard docker\nbuild command uses a builder that does not respect our .dockerignore\nfile, which will lead to a build failure. To build, you should instead\nrun a command like one of these:\n\ndocker/docker-bake-rust-all.sh\ndocker/docker-bake-rust-all.sh indexer\n\nIf you are 100 percent sure you know what you're doing, you can add this flag:\n--build-arg BUILT_VIA_BUILDKIT=true\n\nFor more information, see https://github.com/aptos-labs/aptos-core/pull/2472\n\nThanks!" && false)

COPY --link . /aptos/

RUN --mount=type=cache,target=/aptos/target --mount=type=cache,target=$CARGO_HOME/registry docker/build-rust-all.sh && rm -rf $CARGO_HOME/registry/index

### Validator Image ###
FROM debian-base AS validator

RUN export DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y libssl1.1 ca-certificates && apt-get clean && rm -r /var/lib/apt/lists/*


RUN apt-get update && apt-get install -y bpfcc-tools
#RUN apt-get install -y bpfcc-tools python-bpfcc libbpfcc libbpfcc-dev

### Needed to run debugging tools like perf

# According to https://packages.debian.org/source/sid/bpfcc,
# BCC build dependencies:
#RUN sudo apt-get install -y arping bison clang-format cmake dh-python \
#	  dpkg-dev pkg-kde-tools ethtool flex inetutils-ping iperf \
#	  libbpf-dev libclang-dev libclang-cpp-dev libedit-dev libelf-dev \
#	  libfl-dev libzip-dev linux-libc-dev llvm-dev libluajit-5.1-dev \
#	  luajit python3-netaddr python3-pyroute2 python3-distutils python3
#Get bcc code
#RUN git clone https://github.com/iovisor/bcc.git
#mkdir bcc/build; cd bcc/build
#cmake ..
#make
#sudo make install

RUN apt-get update && apt-get install -y linux-perf sudo procps gdb
RUN apt-get install -y strace htop
### Because build machine perf might not match run machine perf, we have to symlink
### Even if version slightly off, still mostly works
RUN ln -sf /usr/bin/perf_* /usr/bin/perf

RUN addgroup --system --gid 6180 aptos && adduser --system --ingroup aptos --no-create-home --uid 6180 aptos

RUN mkdir -p /opt/aptos/etc
COPY --link --from=builder /aptos/dist/aptos-node /usr/local/bin/
COPY --link --from=builder /aptos/dist/db-backup /usr/local/bin/
COPY --link --from=builder /aptos/dist/db-bootstrapper /usr/local/bin/
COPY --link --from=builder /aptos/dist/db-restore /usr/local/bin/

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
ENV RUST_LOG_FORMAT=json

# add build info
ARG BUILD_DATE
ENV BUILD_DATE ${BUILD_DATE}
ARG GIT_TAG
ENV GIT_TAG ${GIT_TAG}
ARG GIT_BRANCH
ENV GIT_BRANCH ${GIT_BRANCH}
ARG GIT_SHA
ENV GIT_SHA ${GIT_SHA}


### Indexer Image ###

FROM debian-base AS indexer

RUN apt-get update && apt-get install -y libssl1.1 ca-certificates net-tools tcpdump iproute2 netcat libpq-dev \
    && apt-get clean && rm -r /var/lib/apt/lists/*

COPY --link --from=builder /aptos/dist/aptos-indexer /usr/local/bin/aptos-indexer

ENV RUST_LOG_FORMAT=json

# add build info
ARG BUILD_DATE
ENV BUILD_DATE ${BUILD_DATE}
ARG GIT_TAG
ENV GIT_TAG ${GIT_TAG}
ARG GIT_BRANCH
ENV GIT_BRANCH ${GIT_BRANCH}
ARG GIT_SHA
ENV GIT_SHA ${GIT_SHA}


### Node Checker Image ###

FROM debian-base AS node-checker

RUN apt-get update && apt-get install -y libssl1.1 ca-certificates net-tools tcpdump iproute2 netcat libpq-dev \
    && apt-get clean && rm -r /var/lib/apt/lists/*

COPY --link --from=builder /aptos/dist/aptos-node-checker /usr/local/bin/aptos-node-checker

ENV RUST_LOG_FORMAT=json

# add build info
ARG BUILD_DATE
ENV BUILD_DATE ${BUILD_DATE}
ARG GIT_TAG
ENV GIT_TAG ${GIT_TAG}
ARG GIT_BRANCH
ENV GIT_BRANCH ${GIT_BRANCH}
ARG GIT_SHA
ENV GIT_SHA ${GIT_SHA}


### Tools Image ###
FROM debian-base AS tools

RUN echo "deb http://deb.debian.org/debian bullseye main" > /etc/apt/sources.list.d/bullseye.list && \
    echo "Package: *\nPin: release n=bullseye\nPin-Priority: 50" > /etc/apt/preferences.d/bullseye

RUN apt-get update && \
    apt-get --no-install-recommends --yes install wget curl libssl1.1 ca-certificates socat python3-botocore/bullseye awscli/bullseye && \
    apt-get clean && \
    rm -r /var/lib/apt/lists/*

RUN ln -s /usr/bin/python3 /usr/local/bin/python
COPY --link docker/tools/boto.cfg /etc/boto.cfg

RUN wget https://storage.googleapis.com/pub/gsutil.tar.gz -O- | tar --gzip --directory /opt --extract && ln -s /opt/gsutil/gsutil /usr/local/bin
RUN cd /usr/local/bin && wget "https://storage.googleapis.com/kubernetes-release/release/v1.18.6/bin/linux/amd64/kubectl" -O kubectl && chmod +x kubectl

COPY --link --from=builder /aptos/dist/db-bootstrapper /usr/local/bin/db-bootstrapper
COPY --link --from=builder /aptos/dist/db-backup /usr/local/bin/db-backup
COPY --link --from=builder /aptos/dist/db-backup-verify /usr/local/bin/db-backup-verify
COPY --link --from=builder /aptos/dist/db-restore /usr/local/bin/db-restore
COPY --link --from=builder /aptos/dist/aptos /usr/local/bin/aptos
COPY --link --from=builder /aptos/dist/aptos-openapi-spec-generator /usr/local/bin/aptos-openapi-spec-generator
COPY --link --from=builder /aptos/dist/transaction-emitter /usr/local/bin/transaction-emitter

### Get Aptos Move modules bytecodes for genesis ceremony
RUN mkdir -p /aptos-framework/move/build
RUN mkdir -p /aptos-framework/move/modules
COPY --link --from=builder /aptos/aptos-framework/releases/artifacts/current/build /aptos-framework/move/build
COPY --link --from=builder /aptos/aptos-token/releases/artifacts/current/build /aptos-framework/move/build

RUN mv /aptos-framework/move/build/**/bytecode_modules/*.mv /aptos-framework/move/modules
RUN mv /aptos-framework/move/build/AptosFramework/bytecode_modules/dependencies/**/*.mv /aptos-framework/move/modules
RUN rm -rf /aptos-framework/move/build

# add build info
ARG BUILD_DATE
ENV BUILD_DATE ${BUILD_DATE}
ARG GIT_TAG
ENV GIT_TAG ${GIT_TAG}
ARG GIT_BRANCH
ENV GIT_BRANCH ${GIT_BRANCH}
ARG GIT_SHA
ENV GIT_SHA ${GIT_SHA}


### Faucet Image ###
FROM debian-base AS faucet

RUN apt-get update && apt-get install -y libssl1.1 ca-certificates nano net-tools tcpdump iproute2 netcat \
    && apt-get clean && rm -r /var/lib/apt/lists/*

RUN mkdir -p /aptos/client/data/wallet/

COPY --link --from=builder /aptos/dist/aptos-faucet /usr/local/bin/aptos-faucet

#install needed tools
RUN apt-get update && apt-get install -y procps

# Mint proxy listening address
EXPOSE 8000
ENV RUST_LOG_FORMAT=json

# add build info
ARG BUILD_DATE
ENV BUILD_DATE ${BUILD_DATE}
ARG GIT_TAG
ENV GIT_TAG ${GIT_TAG}
ARG GIT_BRANCH
ENV GIT_BRANCH ${GIT_BRANCH}
ARG GIT_SHA
ENV GIT_SHA ${GIT_SHA}


### Forge Image ###

FROM debian-base as forge

RUN apt-get update && apt-get install -y libssl1.1 ca-certificates openssh-client wget busybox git unzip awscli && apt-get clean && rm -r /var/lib/apt/lists/*

RUN mkdir /aptos

# copy helm charts from source
COPY --link --from=builder /aptos/terraform/helm /aptos/terraform/helm

RUN cd /usr/local/bin && wget "https://storage.googleapis.com/kubernetes-release/release/v1.18.6/bin/linux/amd64/kubectl" -O kubectl && chmod +x kubectl
RUN cd /usr/local/bin && wget "https://get.helm.sh/helm-v3.8.0-linux-amd64.tar.gz" -O- | busybox tar -zxvf - && mv linux-amd64/helm . && chmod +x helm
ENV PATH "$PATH:/root/bin"

WORKDIR /aptos
COPY --link --from=builder /aptos/dist/forge /usr/local/bin/forge
ENV RUST_LOG_FORMAT=json

# add build info
ARG BUILD_DATE
ENV BUILD_DATE ${BUILD_DATE}
ARG GIT_TAG
ENV GIT_TAG ${GIT_TAG}
ARG GIT_BRANCH
ENV GIT_BRANCH ${GIT_BRANCH}
ARG GIT_SHA
ENV GIT_SHA ${GIT_SHA}

ENTRYPOINT ["/tini", "--", "forge"]
