### Faucet Image ###
FROM debian-base AS faucet

# Current debian base used in build is bullseye, pin to prevent unexpected changes


RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    sed -i 's|http://security.debian.org/debian-security|https://cloudfront.debian.net/debian-security|g' /etc/apt/sources.list &&  \
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
