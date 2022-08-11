# Aptos Protobufs

This is a simple crate for working with the [Aptos](https://aptos.org) protobufs


## Generating protos
We use [buf](https://docs.buf.build/introduction) to generate protos.

installation is easy on mac
```bash
brew install bufbuild/buf/buf
```
or for linux
```bash
# Substitute BIN for your bin directory.
# Substitute VERSION for the current released version.
BIN="/usr/local/bin" && \
VERSION="1.7.0" && \
  curl -sSL \
    "https://github.com/bufbuild/buf/releases/download/v${VERSION}/buf-$(uname -s)-$(uname -m)" \
    -o "${BIN}/buf" && \
  chmod +x "${BIN}/buf"
```
please check [here](https://docs.buf.build/installation) for other OSes

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
