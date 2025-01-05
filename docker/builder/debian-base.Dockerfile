#syntax=docker/dockerfile:1.4

FROM debian AS debian-base

ARG TARGETARCH

RUN rm -f /etc/apt/apt.conf.d/docker-clean; echo 'Binary::apt::APT::Keep-Downloaded-Packages "true";' > /etc/apt/apt.conf.d/keep-cache

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    sed -i 's|http://deb.debian.org/debian|http://cloudfront.debian.net/debian|g' /etc/apt/sources.list &&  \
    apt-get update && apt-get --no-install-recommends --allow-downgrades -y install \
        ca-certificates \
        curl \
        iproute2 \
        libpq-dev \
        libssl1.1 \
        netcat \
        net-tools \
        tcpdump

# Add Tini to make sure the binaries receive proper SIGTERM signals when Docker is shut down
ADD --chmod=755 https://github.com/krallin/tini/releases/download/v0.19.0/tini-$TARGETARCH /tini
ENTRYPOINT ["/tini", "--"]
