FROM debian:buster-20211011@sha256:f9182ead292f45165f4a851e5ff98ea0800e172ccedce7d17764ffaae5ed4d6e

VOLUME /project
WORKDIR /project

ADD . /move/

#Needed for sccache to function, and to work around home dir being blatted.
ENV CARGO_HOME "/opt/cargo"
ENV RUSTUP_HOME "/opt/rustup"

ENV DOTNET_ROOT "/opt/dotnet"
ENV Z3_EXE "/opt/bin/z3"
ENV CVC5_EXE "/opt/bin/cvc5"
ENV BOOGIE_EXE "/opt/dotnet/tools/boogie"
ENV SOLC_EXE "/opt/bin/solc"
ENV PATH "/opt/cargo/bin:/usr/lib/golang/bin:/opt/bin:${DOTNET_ROOT}:${DOTNET_ROOT}/tools:$PATH"

# Batch mode and all operations tooling
RUN mkdir -p /github/home && \
    mkdir -p /opt/cargo/ && \
    mkdir -p /opt/git/ && \
	sed -i -e 's/\r$//' /move/scripts/*.sh && \
    /move/scripts/dev_setup.sh -t -b -p -y -d -g -n && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Compile a small rust tool?  But we already have in dev_setup (sccache/grcov)...?
# Test that all commands we need are installed and on the PATH
RUN [ -x "$(set -x; command -v rustup)" ] \
    && [ -x "$(set -x; command -v cargo)" ] \
    && [ -x "$(set -x; command -v solc)" ] \
    && [ -x "$(set -x; command -v z3)" ] \
    && [ -x "$(set -x; command -v "$BOOGIE_EXE")" ] \
    && [ -x "$(set -x; xargs rustup which cargo --toolchain < /move/rust-toolchain )" ] \
    && [ -x "$(set -x; command -v clang)" ] \
    && [ -x "$(set -x; command -v npm)" ]

RUN cd /move; cargo build -p move-cli

ENTRYPOINT ["/move/target/debug/move"]
