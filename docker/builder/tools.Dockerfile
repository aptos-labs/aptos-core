### Tools Image ###
FROM debian-base AS tools

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get --no-install-recommends --allow-downgrades -y \
    install \
    wget \
    curl \
    perl-base=5.32.1-4+deb11u4 \
    libtinfo6=6.2+20201114-2+deb11u2 \
    git \
            socat \
    python3-botocore/bullseye \
    awscli/bullseye \
    gnupg2 \
    pigz

RUN echo "deb [signed-by=/usr/share/keyrings/cloud.google.gpg] http://packages.cloud.google.com/apt cloud-sdk main" | tee -a /etc/apt/sources.list.d/google-cloud-sdk.list && \
    curl https://packages.cloud.google.com/apt/doc/apt-key.gpg | apt-key --keyring /usr/share/keyrings/cloud.google.gpg add - && \
    apt-get -y update && \
    apt-get -y install google-cloud-sdk

RUN ln -s /usr/bin/python3 /usr/local/bin/python
COPY --link docker/tools/boto.cfg /etc/boto.cfg

RUN wget https://storage.googleapis.com/pub/gsutil.tar.gz -O- | tar --gzip --directory /opt --extract && ln -s /opt/gsutil/gsutil /usr/local/bin
RUN cd /usr/local/bin && wget "https://storage.googleapis.com/kubernetes-release/release/v1.18.6/bin/linux/amd64/kubectl" -O kubectl && chmod +x kubectl

COPY --link --from=tools-builder /velor/dist/velor-debugger /usr/local/bin/velor-debugger
COPY --link --from=tools-builder /velor/dist/velor /usr/local/bin/velor
COPY --link --from=tools-builder /velor/dist/velor-openapi-spec-generator /usr/local/bin/velor-openapi-spec-generator
COPY --link --from=tools-builder /velor/dist/velor-fn-check-client /usr/local/bin/velor-fn-check-client
COPY --link --from=tools-builder /velor/dist/velor-transaction-emitter /usr/local/bin/velor-transaction-emitter
COPY --link --from=tools-builder /velor/dist/velor-api-tester /usr/local/bin/velor-api-tester

# Copy the example module to publish for api-tester
COPY --link --from=tools-builder /velor/velor-move/framework/velor-framework /velor-move/framework/velor-framework
COPY --link --from=tools-builder /velor/velor-move/framework/velor-stdlib /velor-move/framework/velor-stdlib
COPY --link --from=tools-builder /velor/velor-move/framework/move-stdlib /velor-move/framework/move-stdlib
COPY --link --from=tools-builder /velor/velor-move/move-examples/hello_blockchain /velor-move/move-examples/hello_blockchain

### Get Velor Move releases for genesis ceremony
RUN mkdir -p /velor-framework/move
COPY --link --from=tools-builder /velor/dist/head.mrb /velor-framework/move/head.mrb

# add build info
ARG BUILD_DATE
ENV BUILD_DATE ${BUILD_DATE}
ARG GIT_TAG
ENV GIT_TAG ${GIT_TAG}
ARG GIT_BRANCH
ENV GIT_BRANCH ${GIT_BRANCH}
ARG GIT_SHA
ENV GIT_SHA ${GIT_SHA}
