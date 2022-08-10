# Aptos Protobufs

This is a simple crate for working with the [Aptos](https://aptos.org) protobufs


## Generating protos
We use [buf](https://docs.buf.build/introduction) to generate protos.

```bash
brew install buf
```

Generating the protos requires `protoc`, as well as a few plugins:
```bash
cargo install protoc-gen-prost
cargo install protoc-gen-prost-serde
cargo install protoc-gen-prost-crate
```

Now we can generate the protos:
```bash
buf generate
```