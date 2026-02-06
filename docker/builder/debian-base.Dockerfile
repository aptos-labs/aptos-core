#syntax=docker/dockerfile:1.4

FROM debian AS debian-base

ARG TARGETARCH

RUN rm -f /etc/apt/apt.conf.d/docker-clean; echo 'Binary::apt::APT::Keep-Downloaded-Packages "true";' > /etc/apt/apt.conf.d/keep-cache

# Configure APT sources to use cloudfront mirror (DEB822 format for Trixie+)
RUN rm -f /etc/apt/sources.list
COPY <<EOF /etc/apt/sources.list.d/debian.sources
Types: deb
URIs: http://cloudfront.debian.net/debian
Suites: trixie trixie-updates
Components: main

Types: deb
URIs: https://cloudfront.debian.net/debian-security
Suites: trixie-security
Components: main
EOF

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get --no-install-recommends --allow-downgrades -y install \
        ca-certificates \
        curl \
        iproute2 \
        libpq5 \
        libssl3 \
        netcat-openbsd \
        net-tools \
        tcpdump

# Add Tini to make sure the binaries receive proper SIGTERM signals when Docker is shut down
ADD --chmod=755 https://github.com/krallin/tini/releases/download/v0.19.0/tini-$TARGETARCH /tini
ENTRYPOINT ["/tini", "--"]
