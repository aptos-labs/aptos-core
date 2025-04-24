#syntax=docker/dockerfile:1.5-labs

FROM debian-base as validator-testing-base 

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
        # Needed to run debugging tools like perf
        gdb \
        linux-perf \
        procps \
        sudo \
        # Extra goodies for debugging
        binutils \
        ghostscript \
        git \
        graphviz \
        htop \
        less \
        libjemalloc-dev \
        strace \
        sysstat \
        valgrind \
        vim

FROM node-builder

FROM tools-builder

### Validator Image ###
# We will build a base testing image with the necessary packages and 
# duplicate steps from validator step. This will, however, reduce 
# cache invalidation and reduce build times. 
FROM validator-testing-base  AS validator-testing

RUN addgroup --system --gid 6180 aptos && adduser --system --ingroup aptos --no-create-home --uid 6180 aptos

RUN mkdir -p /opt/aptos/etc
COPY --link --from=node-builder /aptos/dist/aptos-node /usr/local/bin/
#COPY --link --from=tools-builder /aptos/dist/aptos-debugger /usr/local/bin/

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
