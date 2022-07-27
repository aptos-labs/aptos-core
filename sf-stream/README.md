# API

This module provides a SF Streamer for pushing protobuf data from the Aptos Blockchain

## Overview

### Models

Models or types are defined in the `aptos-api-types` package (in the directory `/api/types`).

These types handle deserialization between internal data types and API response JSON types. These are then used to
construct the Protobuf messages.

### Error Handling

All internal errors should be converted into `anyhow::Error` first.

### Unit Test

Handler tests should cover all aspects of features and functions.

A `TestContext` is implemented to create components' stubs that API handlers are connected to.
These stubs are more close to real production components, instead of mocks, so that tests can ensure the handlers are
working well with other components in the systems.
For example, we use real AptosDB implementation in tests for API layers to interact with the database.

Most of the utility functions are provided by the `TestContext`.

### Integration/Smoke Test

Run integration/smoke tests in `testsuite/smoke-test`

```
cargo test --test "forge" "api::"
```

## Aptos Node Operation

The Aptos node generates the following default SF-Stream configuration:

```
sf_stream:
  enabled: false
```

When `sf_stream.enabled` is set to `true`, the SF-Stream will be enabled, and transactions will be streamed to stdout.

## Installing Protobuf Compiler

1. Install the protobuf compiler `protoc`:
   On OS X [Homebrew](https://github.com/Homebrew/brew) can be used:
   
   ```sh
   brew install protobuf
   ```

   On Ubuntu the `protobuf-compiler` package can be installed like so:
   
   ```sh
   apt-get install protobuf-compiler
   ```
2. Install the `protoc` plugin `protoc-gen-rust` with `cargo install protobuf-codegen`

3. Add the `protoc-gen-rust` plugin to your $PATH

   ```sh
   PATH="$HOME/.cargo/bin:$PATH"
   ```

4. Run `protoc` to generate the .rs files:

   ```sh
   protoc --rust_out src/protos src/protos/*.proto
   ```

   This will generate the requisite .rs files in src/protos directory
