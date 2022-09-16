> :warning: DO* NOT USE THIS, IT'S NOT READY. Go to crates/indexer for a working indexer
## Quick Start (Locally)

Use this quickstart guide to set up your environment to use Substreams locally.

First, [copy this repository](https://github.com/streamingfast/substreams-template/generate) and clone it.

## Install Dependencies

### Install Rust

We're going to be using the [Rust programming language](https://www.rust-lang.org/), to develop some custom logic.

There are [several ways to install Rust](https://www.rust-lang.org/tools/install), but for the sake of brevity:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env # to configure your current shell
```

### Protobuf

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

## Obtain the `substreams` CLI tool

### From `brew` (for Mac OS)

```
brew install streamingfast/tap/substreams
```

### Validation

Ensure that `substreams` CLI works as expected:

```
substreams -v
version (...)
```

## Generating Protobuf

NEED MORE INSTRUCTIONS
Go to aptos-protos and run `buf codegen`
## Compile

At this point, we're ready to build our WASM binary and Protobuf definitions.

```bash
make build
```

The resulting WASM artifact will be found at `./target/wasm32-unknown-unknown/release/`

## Run your Substream

We're now ready to run our example Substream!

> Don't forget to be at the root of the project to run the following commands

```bash
make stream_block
make stream_token
```

You can add additional commands in ./Makefile
## Next Steps

Congratulations! You've successfully run a Substream.

- Read the documentation at https://github.com/streamingfast/substreams under _Documentation_.
- Look at [Playground](https://github.com/streamingfast/substreams-playground) for more learning examples.
