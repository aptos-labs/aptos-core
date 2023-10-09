#!/bin/env bash

# Ensure that $TARGETPLATFORM is set
if [ -z "$TARGETPLATFORM" ]; then
  echo "TARGETPLATFORM is not set"
  exit 1
fi

# Ensure that $BUILDPLATFORM is set
if [ -z "$BUILDPLATFORM" ]; then
  echo "BUILDPLATFORM is not set"
  exit 1
fi

# Build platform should only be linux/amd64 or linux/arm64
if [ "$BUILDPLATFORM" != "linux/amd64" ] && [ "$BUILDPLATFORM" != "linux/arm64" ]; then
  echo "BUILDPLATFORM should be linux/amd64 or macos/arm64. Got $BUILDPLATFORM"
  exit 1
fi

# Target platform should only be linux/arm64 or linux/amd64
if [ "$TARGETPLATFORM" != "linux/arm64" ] && [ "$TARGETPLATFORM" != "linux/amd64" ]; then
  echo "TARGETPLATFORM should be linux/arm64 or linux/amd64. Got $TARGETPLATFORM"
  exit 1
fi

# If source platform is linux/arm64 and target platform is linux/amd64, then we need to add the amd64 architecture
if [ "$BUILDPLATFORM" == "linux/arm64" ] && [ "$TARGETPLATFORM" == "linux/amd64" ]; then
  echo "Adding amd64 architecture"
  dpkg --add-architecture amd64

  cp /etc/apt/sources.list ~/sources.list.bak
  (
    (grep ^deb /etc/apt/sources.list | sed 's/deb /deb [arch=arm64] /') && \
    (grep ^deb /etc/apt/sources.list | sed 's/deb /deb [arch=amd64] /g; s/ports\.ubuntu/archive.ubuntu/g; s/ubuntu-ports/ubuntu/g') \
  ) | tee /tmp/sources.list
  mv /tmp/sources.list /etc/apt/sources.list  
  
  rustup target add x86_64-unknown-linux-gnu
  rustup toolchain install stable-x86_64-unknown-linux-gnu

  apt update
  apt install gcc-x86-64-linux-gnu -y
fi
