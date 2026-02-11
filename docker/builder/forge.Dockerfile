### Forge Image ###

FROM debian-base as forge

# 1. Install apt packages (cached via mount)
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install --no-install-recommends -y \
        awscli \
        busybox \
        git \
        openssh-client \
        unzip \
        wget

# 2. Download kubectl and helm (version-pinned, rarely changes - good cache layer)
RUN ARCH=$(dpkg --print-architecture) && \
    wget -q "https://dl.k8s.io/release/v1.35.0/bin/linux/${ARCH}/kubectl" \
        -O /usr/local/bin/kubectl && \
    chmod +x /usr/local/bin/kubectl && \
    wget -q "https://get.helm.sh/helm-v3.20.0-linux-${ARCH}.tar.gz" -O- | \
        busybox tar -zxf - -C /tmp && \
    mv /tmp/linux-${ARCH}/helm /usr/local/bin/helm && \
    chmod +x /usr/local/bin/helm && \
    rm -rf /tmp/linux-${ARCH}

# 3. Copy from builders (changes with code - keep after static layers)
WORKDIR /aptos
COPY --link --from=tools-builder /aptos/terraform/helm /aptos/terraform/helm
COPY --link --from=tools-builder /aptos/testsuite/forge/src/backend/k8s/helm-values/aptos-node-default-values.yaml /aptos/terraform/aptos-node-default-values.yaml
COPY --link --from=tools-builder /aptos/aptos-move/framework/ /aptos/aptos-move/framework/
COPY --link --from=tools-builder /aptos/aptos-move/aptos-release-builder/ /aptos/aptos-move/aptos-release-builder/
COPY --link --from=forge-builder /aptos/dist/forge /usr/local/bin/forge

ENV RUST_LOG_FORMAT=json

# 4. Build info (changes every build - keep last for best caching)
ARG BUILD_DATE
ENV BUILD_DATE ${BUILD_DATE}
ARG GIT_TAG
ENV GIT_TAG ${GIT_TAG}
ARG GIT_BRANCH
ENV GIT_BRANCH ${GIT_BRANCH}
ARG GIT_SHA
ENV GIT_SHA ${GIT_SHA}

ENTRYPOINT ["/tini", "--", "forge"]
