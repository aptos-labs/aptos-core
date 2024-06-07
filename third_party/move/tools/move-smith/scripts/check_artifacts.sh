#!/bin/bash

# This script checks all the raw artifact files
#
# Usage: ./scripts/check_artifacts.sh <artifacts_dir>

MOVE_SMITH_DIR=$(realpath $(dirname $0)/..)
APTOS_DIR=$(realpath $MOVE_SMITH_DIR/../../../..)
CHECK_ARTIFACT=$(realpath $(find $APTOS_DIR/target/ -name "check_artifact"))

artifacts_dir=${1:-"$MOVE_SMITH_DIR/fuzz/artifacts/transactional"}

function check_artifact {
  echo "Checking $(basename $1)"
  $CHECK_ARTIFACT -f $1 > /dev/null 2>&1
  if [ $? -ne 0 ]; then
    echo "Invalid artifact found: $1"
    echo "To reproduce the error, run:"
    echo "$CHECK_ARTIFACT -f $1"
  fi
}

echo "Checking artifacts in $artifacts_dir"

N=8
for i in $artifacts_dir/*; do
    ((a=a%N)); ((a++==0)) && wait
    check_artifact $i &
done

wait
