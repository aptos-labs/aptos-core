#!/bin/sh

# This assumes that cargo, pnpm, poetry, buf, and protoc are already installed.
# The TS plugins are pulled automatically since we depend on them directly from
# the buf.build community plugin registry.

set -e
set -x

# For generating Rust code
cargo install --locked --version 0.4.0 protoc-gen-prost
cargo install --locked --version 0.2.3 protoc-gen-prost-serde
cargo install --locked --version 0.3.1 protoc-gen-prost-crate
cargo install --locked --version 0.4.1 protoc-gen-tonic

# For generating Python code
cd python
poetry install
