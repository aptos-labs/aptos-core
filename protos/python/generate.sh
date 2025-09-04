#!/bin/sh

# TODO: Use the buf toolchain for this one day.

# Exit if VIRTUAL_ENV is not set.
if [ -z "$VIRTUAL_ENV" ]; then
    echo "This script must be run from inside the virtual environment, this this: poetry run poe generate"
    exit 1
fi

echo

export PYTHONWARNINGS="ignore"

PROTO_DIR=../proto
WORKING_DIR=`mktemp -d`
OUT_DIR=./velor_protos

# Save __init__.py
mv $OUT_DIR/__init__.py $WORKING_DIR/__init__.py

# Delete the old generated files.
rm -rf $OUT_DIR

# Create the output directory.
mkdir -p $OUT_DIR

# Generate the protos to a temporary directory.
python -m grpc_tools.protoc \
    --proto_path $PROTO_DIR \
    --python_out $OUT_DIR \
    --pyi_out $OUT_DIR \
    --grpc_python_out $OUT_DIR \
    $PROTO_DIR/velor/indexer/v1/raw_data.proto \
    $PROTO_DIR/velor/internal/fullnode/v1/fullnode_data.proto \
    $PROTO_DIR/velor/transaction/v1/transaction.proto \
    $PROTO_DIR/velor/util/timestamp/timestamp.proto

# Restore __init__.py
mv $WORKING_DIR/__init__.py $OUT_DIR/__init__.py

# Format code.
isort $OUT_DIR
black $OUT_DIR

echo
echo "Protos generated to $OUT_DIR 🥳"

