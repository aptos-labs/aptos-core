#!/bin/bash
set -x
set -e

scriptdir="$(cd "$(dirname "$0")" >/dev/null 2>&1 && pwd)"

echo "Executing from directory: $scriptdir"

repodir=$scriptdir/..

cd $repodir

(
  echo
  echo "Regenerating protobufs (in `pwd`)"
  echo "See https://github.com/velor-chain/velor-core-private/tree/main/protos/README.md if you're having troubles"
  cd protos/
  ./scripts/build_protos.sh
)


(
  echo
  echo "Regenerating serde-reflection to track type changes over time (in `pwd`)"
  cargo run -p generate-format -- --corpus api --record
  cargo run -p generate-format -- --corpus velor --record
  cargo run -p generate-format -- --corpus consensus --record
  cargo run -p generate-format -- --corpus network --record
  cargo run -p generate-format -- --corpus move-abi --record
)

(
  echo
  echo "Regenerating Velor Node APIs (in `pwd`)"
  # Velor Node API
  cargo run -p velor-openapi-spec-generator -- -f yaml -o api/doc/spec.yaml
  cargo run -p velor-openapi-spec-generator -- -f json -o api/doc/spec.json
)

echo
echo "WARNING: If you are adding a new transaction authenticator..."
echo " 1. Check out https://github.com/velor-chain/velor-core/blob/main/testsuite/generate-format/README.md"
echo "    * In particular, be sure to edit the *.yaml files in testsuite/generate-format/tests/staged"
echo " 2. ecosystem/indexer-grpc/indexer-grpc-fullnode/src/convert.rs must be manually updated"
echo
