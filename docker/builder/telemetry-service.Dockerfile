FROM debian-base AS telemetry-service

COPY --link --from=tools-builder /velor/dist/velor-telemetry-service /usr/local/bin/velor-telemetry-service

EXPOSE 8000
ENV RUST_LOG_FORMAT=json

# add build info
ARG GIT_TAG
ENV GIT_TAG ${GIT_TAG}
ARG GIT_BRANCH
ENV GIT_BRANCH ${GIT_BRANCH}
ARG GIT_SHA
ENV GIT_SHA ${GIT_SHA}
