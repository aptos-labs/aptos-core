### Tools Image ###
FROM debian-base AS tools

RUN echo "deb http://deb.debian.org/debian bullseye main" > /etc/apt/sources.list.d/bullseye.list && \
    echo "Package: *\nPin: release n=bullseye\nPin-Priority: 50" > /etc/apt/preferences.d/bullseye

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get --no-install-recommends -y \
    install \
    wget \
    curl \
    libssl1.1 \
    ca-certificates \
    socat \
    python3-botocore/bullseye \
    awscli/bullseye

RUN ln -s /usr/bin/python3 /usr/local/bin/python
COPY --link docker/tools/boto.cfg /etc/boto.cfg

RUN wget https://storage.googleapis.com/pub/gsutil.tar.gz -O- | tar --gzip --directory /opt --extract && ln -s /opt/gsutil/gsutil /usr/local/bin
RUN cd /usr/local/bin && wget "https://storage.googleapis.com/kubernetes-release/release/v1.18.6/bin/linux/amd64/kubectl" -O kubectl && chmod +x kubectl

COPY --link --from=builder /aptos/dist/aptos-db-bootstrapper /usr/local/bin/aptos-db-bootstrapper
COPY --link --from=builder /aptos/dist/db-backup /usr/local/bin/db-backup
COPY --link --from=builder /aptos/dist/db-backup-verify /usr/local/bin/db-backup-verify
COPY --link --from=builder /aptos/dist/db-restore /usr/local/bin/db-restore
COPY --link --from=builder /aptos/dist/aptos /usr/local/bin/aptos
COPY --link --from=builder /aptos/dist/aptos-openapi-spec-generator /usr/local/bin/aptos-openapi-spec-generator
COPY --link --from=builder /aptos/dist/aptos-fn-check-client /usr/local/bin/aptos-fn-check-client
COPY --link --from=builder /aptos/dist/aptos-transaction-emitter /usr/local/bin/aptos-transaction-emitter

### Get Aptos Move releases for genesis ceremony
RUN mkdir -p /aptos-framework/move
COPY --link --from=builder /aptos/dist/head.mrb /aptos-framework/move/head.mrb

# add build info
ARG BUILD_DATE
ENV BUILD_DATE ${BUILD_DATE}
ARG GIT_TAG
ENV GIT_TAG ${GIT_TAG}
ARG GIT_BRANCH
ENV GIT_BRANCH ${GIT_BRANCH}
ARG GIT_SHA
ENV GIT_SHA ${GIT_SHA}