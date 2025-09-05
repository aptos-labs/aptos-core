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

ARG BINARY_PATH=target/release
# Copy the binary and modify its interpreter
COPY --chmod=0755 ${BINARY_PATH}/aptos-node /usr/local/bin/aptos-node
COPY --chmod=0755 ${BINARY_PATH}/aptos /usr/local/bin/aptos
COPY --chmod=0755 ${BINARY_PATH}/l1-migration /usr/local/bin/l1-migration

RUN patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 /usr/local/bin/aptos-node
RUN patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 /usr/local/bin/aptos
RUN patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 /usr/local/bin/l1-migration

# Verify the binary can be executed
RUN LD_LIBRARY_PATH=/lib/x86_64-linux-gnu:/usr/lib/x86_64-linux-gnu /usr/local/bin/aptos-node --version

# Run aptos-node by default
ENTRYPOINT ["/usr/local/bin/aptos-node"]
CMD ["--version"]
