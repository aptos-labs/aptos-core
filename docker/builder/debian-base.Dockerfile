#syntax=docker/dockerfile:1.4

FROM debian AS debian-base

ARG TARGETARCH

RUN rm -f /etc/apt/apt.conf.d/docker-clean; echo 'Binary::apt::APT::Keep-Downloaded-Packages "true";' > /etc/apt/apt.conf.d/keep-cache

# Add Tini to make sure the binaries receive proper SIGTERM signals when Docker is shut down
ADD --chmod=755 https://github.com/krallin/tini/releases/download/v0.19.0/tini-$TARGETARCH /tini
ENTRYPOINT ["/tini", "--"]