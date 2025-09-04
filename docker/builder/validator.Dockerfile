### Validator Image ###

FROM node-builder

FROM tools-builder

FROM debian-base AS validator

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install --no-install-recommends -y \
        # Needed to run debugging tools like perf
        linux-perf \
        sudo \
        procps \
        gdb

### Because build machine perf might not match run machine perf, we have to symlink
### Even if version slightly off, still mostly works
RUN ln -sf /usr/bin/perf_* /usr/bin/perf

RUN addgroup --system --gid 6180 velor && adduser --system --ingroup velor --no-create-home --uid 6180 velor

RUN mkdir -p /opt/velor/etc
COPY --link --from=node-builder /velor/dist/velor-node /usr/local/bin/
COPY --link --from=tools-builder /velor/dist/velor-debugger /usr/local/bin/

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
