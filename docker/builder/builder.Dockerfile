#syntax=docker/dockerfile:1.4

FROM rust as rust-base
WORKDIR /aptos


RUN rm -f /etc/apt/apt.conf.d/docker-clean; echo 'Binary::apt::APT::Keep-Downloaded-Packages "true";' > /etc/apt/apt.conf.d/keep-cache
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    sed -i 's|http://deb.debian.org/debian|http://cloudfront.debian.net/debian|g' /etc/apt/sources.list &&  \
    apt update && apt-get --no-install-recommends install -y \
        binutils \
        clang \
        cmake \
        curl \
        git \
        libdw-dev \
        libpq-dev \
        libssl-dev \
        libudev-dev \
        lld \
        pkg-config

RUN rustup install 1.78.0

### Build Rust code ###
FROM rust-base as builder-base

# Confirm that this Dockerfile is being invoked from an appropriate builder.
# See https://github.com/aptos-labs/aptos-core/pull/2471
# See https://github.com/aptos-labs/aptos-core/pull/2472
ARG BUILT_VIA_BUILDKIT
ENV BUILT_VIA_BUILDKIT $BUILT_VIA_BUILDKIT
RUN test -n "$BUILT_VIA_BUILDKIT" || (printf "===\nREAD ME\n===\n\nYou likely just tried run a docker build using this Dockerfile using\nthe standard docker builder (e.g. docker build). The standard docker\nbuild command uses a builder that does not respect our .dockerignore\nfile, which will lead to a build failure. To build, you should instead\nrun a command like one of these:\n\ndocker/docker-bake-rust-all.sh\ndocker/docker-bake-rust-all.sh indexer\n\nIf you are 100 percent sure you know what you're doing, you can add this flag:\n--build-arg BUILT_VIA_BUILDKIT=true\n\nFor more information, see https://github.com/aptos-labs/aptos-core/pull/2472\n\nThanks!" && false)

# cargo profile and features
ARG PROFILE
ENV PROFILE ${PROFILE}
ARG FEATURES
ENV FEATURES ${FEATURES}
ARG CARGO_TARGET_DIR
ENV CARGO_TARGET_DIR ${CARGO_TARGET_DIR}

RUN ARCHITECTURE=$(uname -m | sed -e "s/arm64/arm_64/g" | sed -e "s/aarch64/aarch_64/g") \
    && curl -LOs "https://github.com/protocolbuffers/protobuf/releases/download/v21.5/protoc-21.5-linux-$ARCHITECTURE.zip" \
    && unzip -o "protoc-21.5-linux-$ARCHITECTURE.zip" -d /usr/local bin/protoc \
    && unzip -o "protoc-21.5-linux-$ARCHITECTURE.zip" -d /usr/local 'include/*' \
    && chmod +x "/usr/local/bin/protoc" \
    && rm "protoc-21.5-linux-$ARCHITECTURE.zip"
RUN --mount=type=secret,id=GIT_CREDENTIALS,target=/root/.git_credentials \
    git config --global credential.helper store

COPY --link . /aptos/

FROM builder-base as aptos-node-builder

RUN --mount=type=secret,id=GIT_CREDENTIALS,target=/root/.git-credentials \
    --mount=type=cache,target=/usr/local/cargo/git,id=node-builder-cargo-git-cache \
    --mount=type=cache,target=/usr/local/cargo/registry,id=node-builder-cargo-registry-cache \
    --mount=type=cache,target=/aptos/target,id=node-builder-target-cache \
    docker/builder/build-node.sh

FROM builder-base as tools-builder

ENV MOVE_COMPILER_V2=true
ENV MOVE_LANGUAGE_V2=true
RUN --mount=type=secret,id=GIT_CREDENTIALS,target=/root/.git-credentials \
    --mount=type=cache,target=/usr/local/cargo/git,id=tools-builder-cargo-git-cache \
    --mount=type=cache,target=/usr/local/cargo/registry,id=tools-builder-cargo-registry-cache \
    --mount=type=cache,target=/aptos/target,id=tools-builder-target-cache \
    docker/builder/build-tools.sh

FROM builder-base as indexer-builder

RUN --mount=type=secret,id=GIT_CREDENTIALS,target=/root/.git-credentials \
    --mount=type=cache,target=/usr/local/cargo/git,id=indexer-builder-cargo-git-cache \
    --mount=type=cache,target=/usr/local/cargo/registry,id=indexer-builder-cargo-registry-cache \
    --mount=type=cache,target=/aptos/target,id=indexer-builder-target-cache \
    docker/builder/build-indexer.sh
