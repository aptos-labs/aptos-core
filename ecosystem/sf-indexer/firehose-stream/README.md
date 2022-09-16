> :warning: DO* NOT USE THIS, IT'S NOT READY. Go to crates/indexer for a working indexer
# API

This module provides a StreamingFast Firehose Streamer for pushing protobuf data from the Aptos Blockchain

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

The Aptos node generates the following default Firehose-Stream configuration:

```
firehose_stream:
  enabled: false
```

When `firehose_stream.enabled` is set to `true`, the Firehose-Stream will be enabled, and transactions will be streamed to stdout.

## Installing Protobuf Compiler

#### Install `protoc`

protoc is a Protocol Buffer compiler. It is needed to generate code for Rust and other languages, out of the protobuf definitions you will create or get through third-party Substreams packages.

There are multiple ways on how to do it. Here is the official documentation of [protocol buffer compiler](https://grpc.io/docs/protoc-installation/).

#### Install `protoc-gen-prost`

This tool helps you render Rust structures out of protobuf definitions, for use in your Substreams modules. It is called by protoc following their plugin system.
Install it with:

```bash
  cargo install protoc-gen-prost
```

> If you forget to install `protoc`, when generating the definitions, you might see error about `cmake` not defined, this is a fallback when `protoc` is not found.

### Install `buf`

[https://buf.build](https://buf.build) is a tool used to simplify the generation of typed structures in any language. It invokes `protoc` and simplifies a good number of things. Substreams packages are compatible with [buf Images](https://docs.buf.build/reference/images).

See the [installation instructions here](https://docs.buf.build/installation).

### Build proto

cargo build

## Testing

### Connect to Firehose

To test with firehose, we need to build aptos-node

```
cd ../aptos-node
cargo install --path .
```

If necessary, set path to aptos-node

```
export PATH={path to directory containing aptos-core repo}:$PATH
```

Then follow instructions in https://github.com/streamingfast/firehose-aptos
