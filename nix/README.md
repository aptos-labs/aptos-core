# Aptos Core Nix Build System

This directory contains the Nix expressions for building and developing Aptos Core. Nix has been adopted as the primary build system for Aptos Core to ensure reproducible builds, streamline dependency management, and provide a consistent development environment, particularly addressing common issues like RocksDB compilation.

## Overview

The Nix build system provides:

1. **Reproducible builds** - Ensures consistent builds across different environments, from local development to CI/CD.
2. **Dependency management** - Automatically handles all system dependencies, ensuring all developers use the same versions.
3. **Consistent Development Environment** - Provides a hermetic and consistent development environment, reducing "it works on my machine" issues.
4. **RocksDB compatibility** - Specifically addresses and fixes RocksDB compilation issues by managing proper environment variables and dependencies.

## Directory Structure

```
nix/
├── default.nix       # Legacy Nix entry point (deprecated, use flake.nix)
├── flake.nix         # Modern Nix flake entry point
├── flake.lock        # Locked dependencies for flake.nix
├── shell.nix         # Development environment definition
├── build-aptos.sh    # Convenience script for building Aptos Core using Nix
├── README.md         # This file
└── pkgs/
    └── aptos-core.nix # Aptos Core package definition
```

## Usage

### Using Nix Flakes (Recommended)

Nix Flakes provide a modern, reproducible way to interact with the Nix build system.

To build Aptos Core:

```bash
nix build .#aptos-core
```

To enter the development environment:

```bash
nix develop
```

To run a specific app (e.g., `aptos-node`):

```bash
nix run .#aptos-node
```

### Using the Build Script

For convenience, you can use the provided `build-aptos.sh` script, which wraps the `nix build` command:

```bash
./nix/build-aptos.sh
```

## Configuration

The Nix build system automatically configures the necessary environment variables to ensure smooth compilation, especially for dependencies like RocksDB and OpenSSL. These include:

- `ROCKSDB_LIB_DIR` - Points to the Nix-managed RocksDB library directory.
- `ROCKSDB_STATIC` - Forces static linking of RocksDB.
- `OPENSSL_NO_VENDOR` - Prevents vendoring OpenSSL, relying on the Nix-provided version.
- `OPENSSL_DIR` - Points to the Nix-managed OpenSSL development directory.

## Troubleshooting

### RocksDB Compilation Issues

If you encounter RocksDB compilation errors, ensure that your Nix environment is correctly set up. The Nix build system is designed to mitigate these issues by providing a controlled environment. If problems persist, verify your Nix installation and flake configuration.

### OpenSSL Issues

Similar to RocksDB, OpenSSL issues are typically resolved by the Nix environment. Ensure your Nix setup is healthy if you face persistent problems.

## Contributing

To update the `flake.lock` file (which locks all input dependencies):

```bash
nix flake update
```

To add new dependencies or modify existing ones, update `flake.nix` and then run `nix flake update` to refresh the lock file.