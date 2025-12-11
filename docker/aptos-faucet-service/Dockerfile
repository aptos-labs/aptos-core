FROM docker.io/library/ubuntu:24.04

# Add build argument for binary path

# Install necessary dependencies
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

# Set the working directory
WORKDIR /app

ARG BINARY_PATH=target/debug
# Copy the binary and modify its interpreter
COPY --chmod=0755 ${BINARY_PATH}/aptos-faucet-service /usr/local/bin/aptos-faucet-service

RUN patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 /usr/local/bin/aptos-faucet-service


# add build info
ARG BUILD_DATE
ENV BUILD_DATE=${BUILD_DATE}
ARG GIT_TAG
ENV GIT_TAG=${GIT_TAG}
ARG GIT_BRANCH
ENV GIT_BRANCH=${GIT_BRANCH}
ARG GIT_SHA
ENV GIT_SHA=${GIT_SHA}

# Verify the binary can be executed
RUN LD_LIBRARY_PATH=/lib/x86_64-linux-gnu:/usr/lib/x86_64-linux-gnu /usr/local/bin/aptos-faucet-service -h

# Run aptos-node by default
ENTRYPOINT ["/usr/local/bin/aptos-faucet-service"]
CMD ["-h"]
