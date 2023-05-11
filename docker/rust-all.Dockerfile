#syntax=docker/dockerfile:1.4

FROM debian:bullseye@sha256:32888a3c745e38e72a5f49161afc7bb52a263b8f5ea1b3b4a6af537678f29491 AS debian-base

# Add Tini to make sure the binaries receive proper SIGTERM signals when Docker is shut down
ADD https://github.com/krallin/tini/releases/download/v0.19.0/tini /tini
RUN chmod +x /tini
ENTRYPOINT ["/tini", "--"]

FROM rust:1.66.1-bullseye@sha256:f72949bcf1daf8954c0e0ed8b7e10ac4c641608f6aa5f0ef7c172c49f35bd9b5 AS rust-base
WORKDIR /aptos
RUN apt-get update && apt-get install -y cmake curl clang git pkg-config libssl-dev libpq-dev
RUN apt-get update && apt-get install binutils lld

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
ARG GIT_CREDENTIALS
ENV GIT_CREDENTIALS ${GIT_CREDENTIALS}

RUN GIT_CREDENTIALS="$GIT_CREDENTIALS" git config --global credential.helper store && echo "${GIT_CREDENTIALS}" > ~/.git-credentials
RUN PROFILE=$PROFILE FEATURES=$FEATURES docker/build-rust-all.sh && rm -rf $CARGO_HOME && rm -rf target
RUN rm -rf ~/.git-credentials

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
COPY --link --from=builder /aptos/dist/aptos-db-tool /usr/local/bin/
COPY --link --from=builder /aptos/dist/aptos-db-bootstrapper /usr/local/bin/

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

RUN apt-get update && apt-get --no-install-recommends --allow-downgrades -y \
    install \
    wget \
    curl \
    perl-base=5.32.1-4+deb11u1 \
    libtinfo6=6.2+20201114-2+deb11u1 \
    git \
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

COPY --link --from=builder /aptos/dist/aptos-db-bootstrapper /usr/local/bin/aptos-db-bootstrapper
COPY --link --from=builder /aptos/dist/aptos-db-tool /usr/local/bin/aptos-db-tool
COPY --link --from=builder /aptos/dist/aptos /usr/local/bin/aptos
COPY --link --from=builder /aptos/dist/aptos-openapi-spec-generator /usr/local/bin/aptos-openapi-spec-generator
COPY --link --from=builder /aptos/dist/aptos-fn-check-client /usr/local/bin/aptos-fn-check-client
COPY --link --from=builder /aptos/dist/aptos-transaction-emitter /usr/local/bin/aptos-transaction-emitter

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

COPY --link --from=builder /aptos/dist/aptos-faucet-service /usr/local/bin/aptos-faucet-service

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
### Get Aptos Framework Release for forge framework upgrade testing
COPY --link --from=builder /aptos/aptos-move/framework/ /aptos/aptos-move/framework/

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
    curl \
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

### Indexer GRPC Image ###

FROM debian-base AS indexer-grpc

RUN apt-get update && apt-get install -y \
    libssl1.1 \
    ca-certificates \
    net-tools \
    tcpdump \
    iproute2 \
    netcat \
    libpq-dev \
    curl \
    && apt-get clean && rm -r /var/lib/apt/lists/*

COPY --link --from=builder /aptos/dist/aptos-indexer-grpc-cache-worker /usr/local/bin/aptos-indexer-grpc-cache-worker
COPY --link --from=builder /aptos/dist/aptos-indexer-grpc-file-store /usr/local/bin/aptos-indexer-grpc-file-store
COPY --link --from=builder /aptos/dist/aptos-indexer-grpc-data-service /usr/local/bin/aptos-indexer-grpc-data-service
COPY --link --from=builder /aptos/dist/aptos-indexer-grpc-parser /usr/local/bin/aptos-indexer-grpc-parser

# The health check port
EXPOSE 8080
# The gRPC port
EXPOSE 50501

ENV RUST_LOG_FORMAT=json

# add build info
ARG GIT_TAG
ENV GIT_TAG ${GIT_TAG}
ARG GIT_BRANCH
ENV GIT_BRANCH ${GIT_BRANCH}
ARG GIT_SHA
ENV GIT_SHA ${GIT_SHA}

### EXPERIMENTAL ###

FROM debian-base as validator-testing-base

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
    # Extra goodies for debugging
    less \
    git \
    vim \
    nano \
    libjemalloc-dev \
    binutils \
    graphviz \
    ghostscript \
    strace \
    htop \
    sysstat \
    valgrind \
    && apt-get clean && rm -r /var/lib/apt/lists/*

# Install pyroscope for profiling
RUN curl https://dl.pyroscope.io/release/pyroscope_0.36.0_amd64.deb --output pyroscope_0.36.0_amd64.deb && apt-get install ./pyroscope_0.36.0_amd64.deb

### Because build machine perf might not match run machine perf, we have to symlink
### Even if version slightly off, still mostly works
RUN ln -sf /usr/bin/perf_* /usr/bin/perf

RUN echo "deb http://deb.debian.org/debian sid main contrib non-free" >> /etc/apt/sources.list
RUN echo "deb-src http://deb.debian.org/debian sid main contrib non-free" >> /etc/apt/sources.list

RUN apt-get update && apt-get install -y \
    arping bison clang-format cmake dh-python \
    dpkg-dev pkg-kde-tools ethtool flex inetutils-ping iperf \
    libbpf-dev libclang-11-dev libclang-cpp-dev libedit-dev libelf-dev \
    libfl-dev libzip-dev linux-libc-dev llvm-11-dev libluajit-5.1-dev \
    luajit python3-netaddr python3-pyroute2 python3-distutils python3 \
    && apt-get clean && rm -r /var/lib/apt/lists/*

RUN git clone https://github.com/aptos-labs/bcc.git
RUN mkdir bcc/build
WORKDIR bcc/
RUN git checkout 5258d14cb35ba08a8757a68386bebc9ea05f00c9
WORKDIR build/
RUN cmake ..
RUN make
RUN make install
WORKDIR ..

### Validator Image ###
# We will build a base testing image with the necessary packages and
# duplicate steps from validator step. This will, however, reduce
# cache invalidation and reduce build times.
FROM validator-testing-base  AS validator-testing

RUN addgroup --system --gid 6180 aptos && adduser --system --ingroup aptos --no-create-home --uid 6180 aptos

RUN mkdir -p /opt/aptos/etc
COPY --link --from=builder /aptos/dist/aptos-node /usr/local/bin/
COPY --link --from=builder /aptos/dist/aptos-db-tool /usr/local/bin/
COPY --link --from=builder /aptos/dist/aptos-db-bootstrapper /usr/local/bin/

# Admission control
EXPOSE 8000
# Validator network
EXPOSE 6180
# Metrics
EXPOSE 9101
# Backup
EXPOSE 6186

# add build info
ARG BUILD_DATE
ENV BUILD_DATE ${BUILD_DATE}
ARG GIT_TAG
ENV GIT_TAG ${GIT_TAG}
ARG GIT_BRANCH
ENV GIT_BRANCH ${GIT_BRANCH}
ARG GIT_SHA
ENV GIT_SHA ${GIT_SHA}

# Capture backtrace on error
ENV RUST_BACKTRACE 1
ENV RUST_LOG_FORMAT=json
