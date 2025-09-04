FROM gcr.io/oss-fuzz-base/base-builder-rust
RUN apt-get update && apt-get install -y make autoconf automake libclang-dev libtool pkg-config
RUN git clone --depth=1 https://github.com/velor-chain/velor-core.git
WORKDIR velor-core
COPY build.sh $SRC
