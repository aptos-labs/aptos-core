#!/bin/sh

# This assumes that cargo, pnpm, poetry, buf, and protoc are already installed.

# For generating Rust code
cargo install --version 0.2.3 protoc-gen-prost
cargo install --version 0.2.3 protoc-gen-prost-serde
cargo install --version 0.3.1 protoc-gen-prost-crate
cargo install --version 0.3.0 protoc-gen-tonic

# For generating TS code
pnpm install -g protoc-gen-ts@0.8.7

# For generating Python code
cd python
poetry install

