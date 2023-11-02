#!/bin/bash

set -e

# Make sure pre-commit is installed.
if ! command -v pre-commit &> /dev/null
then
    echo "pre-commit could not be found. Please install it with 'pip install pre-commit'."
    exit
fi

# Make sure buf is installed.
if ! command -v buf &> /dev/null
then
    echo "buf could not be found. Please install it with 'brew install buf'."
    exit
fi

# Make sure poetry is installed.
if ! command -v poetry &> /dev/null
then
    echo "poetry could not be found. Please install it with 'pip install poetry'."
    exit
fi

# Change to the parent directory.
cd "$(dirname "$0")"
cd ..

# Generate code for Rust and TS.
for file in *.gen.yaml
do
    # For Python we use the Python toolchain.
    if [[ $file == *"python"* ]]; then
        continue
    fi
    buf generate --template "$file"
done

# Generate code for Python. Currently there is no easy way to use buf for Python
# without using a remote registry or compiling github.com/grpc/grpc from source,
# so instead we use the Python toolchain, specifically grpc_tools.protoc.
cd python
poetry install
poetry run poe generate

# Run the pre-commit steps.
pre-commit run --all-files > /dev/null || true
