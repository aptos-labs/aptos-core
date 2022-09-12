#syntax=docker/dockerfile:1.4

FROM debian:buster-20220822@sha256:faa416b9eeda2cbdb796544422eedd698e716dbd99841138521a94db51bf6123 AS debian-base

# Add Tini to make sure the binaries receive proper SIGTERM signals when Docker is shut down
ADD https://github.com/krallin/tini/releases/download/v0.19.0/tini /tini
RUN chmod +x /tini
ENTRYPOINT ["/tini", "--"]

FROM rust:1.63.0-buster@sha256:0110d1b4193029735f1db1c0ed661676ed4b6f705b11b1ebe95c655b52e6906f AS rust-base
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

RUN ARCHITECTURE=$(uname -m | sed -e "s/arm64/arm_64/g" | sed -e "s/aarch64/aarch_64/g") \
    && curl -LOs "https://github.com/protocolbuffers/protobuf/releases/download/v21.5/protoc-21.5-linux-$ARCHITECTURE.zip" \
    && unzip -o "protoc-21.5-linux-$ARCHITECTURE.zip" -d /usr/local bin/protoc \
    && unzip -o "protoc-21.5-linux-$ARCHITECTURE.zip" -d /usr/local 'include/*' \
    && chmod +x "/usr/local/bin/protoc" \
    && rm "protoc-21.5-linux-$ARCHITECTURE.zip"

# cargo profile and features
ARG PROFILE
ENV PROFILE ${PROFILE}
ARG FEATURES
ENV FEATURES ${FEATURES}

RUN PROFILE=$PROFILE FEATURES=$FEATURES docker/build-rust-all.sh && rm -rf $CARGO_HOME && rm -rf target 

### Validator Image ###
FROM debian-base AS validator

RUN apt-get update && apt-get install -y \
    libssl1.1 \
    ca-certificates \
    # Needed to run debugging tools like perf
    linux-perf \
    sudo \
    procps \
    gdb \
    curl \
    # postgres client lib required for indexer
    libpq-dev \
    && apt-get clean && rm -r /var/lib/apt/lists/*

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

### Node Checker Image ###

FROM debian-base AS node-checker

RUN apt-get update && apt-get install -y \
    libssl1.1 \
    ca-certificates \
    net-tools \
    tcpdump \
    iproute2 \
    netcat \
    libpq-dev \
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

RUN apt-get update && apt-get --no-install-recommends -y \
    install \
    wget \
    curl \
    libssl1.1 \
    ca-certificates \
    socat \
    python3-botocore/bullseye \
    awscli/bullseye \ 
    && apt-get clean && rm -r /var/lib/apt/lists/*

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
COPY --link --from=builder /aptos/dist/aptos-fn-check-client /usr/local/bin/aptos-fn-check-client
COPY --link --from=builder /aptos/dist/transaction-emitter /usr/local/bin/transaction-emitter

### Get Aptos Move releases for genesis ceremony
RUN mkdir -p /aptos-framework/move
COPY --link --from=builder /aptos/dist/head.mrb /aptos-framework/move/head.mrb

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

RUN apt-get update && apt-get install -y \
    libssl1.1 \
    ca-certificates \
    nano \
    net-tools \
    tcpdump \
    iproute2 \
    netcat \
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

RUN apt-get update && apt-get install -y libssl1.1 \
    ca-certificates \
    openssh-client \
    wget \
    busybox \
    git \
    unzip \
    awscli \
    && apt-get clean && rm -r /var/lib/apt/lists/*

RUN mkdir /aptos

# copy helm charts from source
COPY --link --from=builder /aptos/terraform/helm /aptos/terraform/helm
COPY --link --from=builder /aptos/testsuite/forge/src/backend/k8s/helm-values/aptos-node-default-values.yaml /aptos/terraform/aptos-node-default-values.yaml

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

### Telemetry Service Image ###

FROM debian-base AS telemetry-service

RUN apt-get update && apt-get install -y \
    libssl1.1 \
    ca-certificates \
    net-tools \
    tcpdump \
    iproute2 \
    netcat \
    libpq-dev \
    && apt-get clean && rm -r /var/lib/apt/lists/*

COPY --link --from=builder /aptos/dist/aptos-telemetry-service /usr/local/bin/aptos-telemetry-service

EXPOSE 8000
ENV RUST_LOG_FORMAT=json

# add build info
ARG GIT_TAG
ENV GIT_TAG ${GIT_TAG}
ARG GIT_BRANCH
ENV GIT_BRANCH ${GIT_BRANCH}
ARG GIT_SHA
ENV GIT_SHA ${GIT_SHA}


### EXPERIMENTAL ###

### Validator Image ###
FROM validator AS validator-testing

RUN apt-get update && apt-get install -y \
    # Extra goodies for debugging
    less \
    vim \
    nano \
    libjemalloc-dev \
    binutils \
    graphviz \
    ghostscript \
    strace \
    htop \
    valgrind \
    bpfcc-tools \
    python-bpfcc \
    libbpfcc \
    libbpfcc-dev \
    && apt-get clean && rm -r /var/lib/apt/lists/*

# Capture backtrace on error
ENV RUST_BACKTRACE 1
ENV RUST_LOG_FORMAT=json
