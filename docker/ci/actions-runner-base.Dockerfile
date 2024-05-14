FROM ghcr.io/actions/actions-runner:2.316.1

# COPY scripts/dev_setup.sh scripts/dev_setup.sh
# COPY rust-toolchain.toml rust-toolchain.toml

# RUN sudo apt-get update -y && sudo apt-get install -y git

# RUN scripts/dev_setup.sh -b -k

ENV DEBIAN_FRONTEND=noninteractive
RUN sudo apt-get update -y \
    && sudo apt-get install -y software-properties-common \
    && sudo add-apt-repository -y ppa:git-core/ppa \
    && sudo apt-get update -y \
    && sudo apt-get install -y --no-install-recommends \
    build-essential \
    curl \
    ca-certificates \
    dnsutils \
    ftp \
    git \
    iproute2 \
    iputils-ping \
    jq \
    libunwind8 \
    locales \
    netcat \
    openssh-client \
    parallel \
    python3-pip \
    rsync \
    shellcheck \
    sudo \
    telnet \
    time \
    tzdata \
    unzip \
    upx \
    wget \
    zip \
    zstd \
    && sudo ln -sf /usr/bin/python3 /usr/bin/python \
    && sudo ln -sf /usr/bin/pip3 /usr/bin/pip \
    && sudo rm -rf /var/lib/apt/lists/*
