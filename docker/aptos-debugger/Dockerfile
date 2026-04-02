FROM docker.io/library/ubuntu:24.04

# Install necessary dependencies (same base as aptos-node)
RUN apt-get update && apt-get install -y \
libjemalloc2 \
libdw1 \
librocksdb-dev \
libssl3 \
libstdc++6 \
zlib1g \
ca-certificates \
patchelf \
&& rm -rf /var/lib/apt/lists/* \
&& ln -sf /usr/lib/x86_64-linux-gnu/librocksdb.so /usr/lib/x86_64-linux-gnu/librocksdb.so.10

WORKDIR /app

ARG BINARY_PATH=target/release
COPY --chmod=0755 ${BINARY_PATH}/aptos-debugger /usr/local/bin/aptos-debugger
RUN patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 /usr/local/bin/aptos-debugger

# Add build info
ARG BUILD_DATE
ENV BUILD_DATE=${BUILD_DATE}
ARG GIT_TAG
ENV GIT_TAG=${GIT_TAG}
ARG GIT_BRANCH
ENV GIT_BRANCH=${GIT_BRANCH}
ARG GIT_SHA
ENV GIT_SHA=${GIT_SHA}

# Verify the binary can be executed
RUN LD_LIBRARY_PATH=/lib/x86_64-linux-gnu:/usr/lib/x86_64-linux-gnu /usr/local/bin/aptos-debugger --help

ENTRYPOINT ["/usr/local/bin/aptos-debugger"]
CMD ["--help"]
