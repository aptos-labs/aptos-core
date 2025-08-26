# Aptos Node Docker Setup

This directory contains multiple approaches for building Docker images with the Aptos node binary.

## Available Build Scripts

### 1. `build-docker.sh` (Current Working Solution)
**Status: ✅ Working**
- Uses Nix development environment to build the binary
- Applies `patchelf` to fix dynamic linker compatibility
- Creates Ubuntu-based Docker image with all required libraries
- **Pros**: Reliable, handles all dependencies
- **Cons**: Requires patching during build process

```bash
./docker/aptos-node/build-docker.sh
docker run --rm aptos-node:latest --help
```

### 2. `build-docker-musl.sh` (Root Cause Solution)
**Status: 🧪 Experimental**
- Builds with `x86_64-unknown-linux-musl` target for maximum portability
- Should produce statically linked binary that works anywhere
- Uses minimal Ubuntu base image
- **Pros**: Addresses root cause, no patching needed
- **Cons**: May have compilation issues with some dependencies

```bash
./docker/aptos-node/build-docker-musl.sh
docker run --rm aptos-node:musl-latest --help
```

### 3. `build-docker-nix.sh` (Nix Package Approach)
**Status: 🚧 Incomplete**
- Would use proper Nix package definition
- Requires setting up `outputHashes` for git dependencies
- **Pros**: Most "correct" Nix approach
- **Cons**: Complex setup, needs Cargo.lock hash management

## Root Cause Solutions

The fundamental issue is that Nix builds binaries with hardcoded paths to Nix store libraries. Here are the proper solutions:

### Option 1: Musl Static Linking (Recommended)
Build with musl target to create portable binaries:
```bash
# Add musl target
rustup target add x86_64-unknown-linux-musl

# Build with musl
cargo build --package aptos-node --target x86_64-unknown-linux-musl
```

### Option 2: Nix Package with Proper Dependencies
Create a proper Nix package that handles all dependencies correctly (requires more setup).

### Option 3: Cross-compilation Environment
Set up a cross-compilation environment that targets the desired glibc version.

## Current Recommendation

For immediate use: **Use `build-docker.sh`** - it's the most reliable working solution.

For production/long-term: **Implement `build-docker-musl.sh`** - it addresses the root cause by building truly portable binaries.

## Dockerfiles

- `Dockerfile` - Full Ubuntu setup with all libraries (used by `build-docker.sh`)
- `Dockerfile.nix` - Minimal setup for portable binaries (used by musl approach)

## Dependencies

The Aptos node requires these runtime libraries:
- libjemalloc.so.2
- libdw.so.1  
- librocksdb.so.10
- libssl.so.3
- libcrypto.so.3
- Standard glibc libraries

The musl approach aims to statically link most of these to avoid runtime dependencies.