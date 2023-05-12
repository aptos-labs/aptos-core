### Faucet Image ###
FROM debian-base AS faucet

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get --no-install-recommends install -y \
        libssl1.1 \
        ca-certificates \
        nano \
        net-tools \
        tcpdump \
        iproute2 \
        netcat \
        procps

RUN mkdir -p /aptos/client/data/wallet/

COPY --link --from=tools-builder /aptos/dist/aptos-faucet-service /usr/local/bin/aptos-faucet-service

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
