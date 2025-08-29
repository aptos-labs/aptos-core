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

COPY --link --from=tools-builder /aptos/dist/aptos-debugger /usr/local/bin/aptos-debugger
COPY --link --from=tools-builder /aptos/dist/aptos /usr/local/bin/aptos
COPY --link --from=tools-builder /aptos/dist/aptos-openapi-spec-generator /usr/local/bin/aptos-openapi-spec-generator
COPY --link --from=tools-builder /aptos/dist/aptos-fn-check-client /usr/local/bin/aptos-fn-check-client
COPY --link --from=tools-builder /aptos/dist/aptos-transaction-emitter /usr/local/bin/aptos-transaction-emitter
COPY --link --from=tools-builder /aptos/dist/aptos-api-tester /usr/local/bin/aptos-api-tester

# Copy replay verification config file
COPY --link --from=tools-builder /aptos/testsuite/replay-verify/replay_verification_config.json /usr/local/share/replay_verification_config.json

# Copy the example module to publish for api-tester
COPY --link --from=tools-builder /aptos/aptos-move/framework/aptos-framework /aptos-move/framework/aptos-framework
COPY --link --from=tools-builder /aptos/aptos-move/framework/aptos-stdlib /aptos-move/framework/aptos-stdlib
COPY --link --from=tools-builder /aptos/aptos-move/framework/move-stdlib /aptos-move/framework/move-stdlib
COPY --link --from=tools-builder /aptos/aptos-move/move-examples/hello_blockchain /aptos-move/move-examples/hello_blockchain

### Get Aptos Move releases for genesis ceremony
RUN mkdir -p /aptos-framework/move
COPY --link --from=tools-builder /aptos/dist/head.mrb /aptos-framework/move/head.mrb

# add build info
ARG BUILD_DATE
ENV BUILD_DATE ${BUILD_DATE}
ARG GIT_TAG
ENV GIT_TAG ${GIT_TAG}
ARG GIT_BRANCH
ENV GIT_BRANCH ${GIT_BRANCH}
ARG GIT_SHA
ENV GIT_SHA ${GIT_SHA}
