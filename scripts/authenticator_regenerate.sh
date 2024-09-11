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
  echo "See https://github.com/aptos-labs/aptos-core-private/tree/main/protos/README.md if you're having troubles"
  cd protos/
  ./scripts/build_protos.sh
)


(
  echo
  echo "Regenerating serde-reflection to track type changes over time (in `pwd`)"
  cargo run -p generate-format -- --corpus api --record
  cargo run -p generate-format -- --corpus aptos --record
  cargo run -p generate-format -- --corpus consensus --record
  cargo run -p generate-format -- --corpus network --record
  cargo run -p generate-format -- --corpus move-abi --record
)

(
  echo
  echo "Regenerating Aptos Node APIs (in `pwd`)"
  # Aptos Node API
  cargo run -p aptos-openapi-spec-generator -- -f yaml -o api/doc/spec.yaml
  cargo run -p aptos-openapi-spec-generator -- -f json -o api/doc/spec.json
)

## Disabled due to errors, and it's for the V1 which is going to be deprecated.
# (
#   echo
#   echo "Regenerating Typescript SDK (in `pwd`)"
#   # Typescript SDK client files
#   cd ecosystem/typescript/sdk
#   pnpm install
#   pnpm generate-client
#
#   # Typescript SDK docs
#   pnpm generate-ts-docs
#   cd ..
# )

echo
echo "WARNING: If you are adding a new transaction authenticator..."
echo " 1. Check out https://github.com/aptos-labs/aptos-core/blob/main/testsuite/generate-format/README.md"
echo "    * In particular, be sure to edit the *.yaml files in testsuite/generate-format/tests/staged"
echo " 2. ecosystem/indexer-grpc/indexer-grpc-fullnode/src/convert.rs must be manually updated"
echo
