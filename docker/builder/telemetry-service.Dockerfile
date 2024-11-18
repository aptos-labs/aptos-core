FROM debian-base AS telemetry-service

# Current debian base used in build is bullseye, pin to prevent unexpected changes
RUN echo "deb https://cloudfront.debian.net/debian/ bullseye main contrib" > /etc/apt/sources.list.d/bullseye.list && \
    echo "Package: *\nPin: release n=bullseye\nPin-Priority: 50" > /etc/apt/preferences.d/bullseye

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \   
    apt-get update && apt-get install --no-install-recommends -y \    
        libssl1.1 \
        ca-certificates \
        net-tools \
        tcpdump \
        iproute2 \
        netcat \
        libpq-dev \
        curl

COPY --link --from=tools-builder /aptos/dist/aptos-telemetry-service /usr/local/bin/aptos-telemetry-service

EXPOSE 8000
ENV RUST_LOG_FORMAT=json

# add build info
ARG GIT_TAG
ENV GIT_TAG ${GIT_TAG}
ARG GIT_BRANCH
ENV GIT_BRANCH ${GIT_BRANCH}
ARG GIT_SHA
ENV GIT_SHA ${GIT_SHA}
