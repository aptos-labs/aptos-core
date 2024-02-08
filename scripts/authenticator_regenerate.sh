#!/bin/bash
set -x

scriptdir="$(cd "$(dirname "$0")" >/dev/null 2>&1 && pwd)"

echo "Executing from directory: $scriptdir"

repodir=$scriptdir/..

(
  echo
  echo "Regenerating serde-reflection to track type changes over time"
  cargo run -p generate-format -- --corpus api --record
  cargo run -p generate-format -- --corpus aptos --record
  cargo run -p generate-format -- --corpus consensus --record
  cargo run -p generate-format -- --corpus network --record
  cargo run -p generate-format -- --corpus move-abi --record
)

(
  echo
  echo "Regenerating protobufs"
  cd $repodir/protos/
  ./scripts/build_protos.sh
)

(
  echo
  echo "Regenerating Aptos Node APIs"
  # Aptos Node API
  cargo run -p aptos-openapi-spec-generator -- -f yaml -o api/doc/spec.yaml
  cargo run -p aptos-openapi-spec-generator -- -f json -o api/doc/spec.json

  echo
  echo "Regenerating Typescript SDK"
  # Typescript SDK client files
  cd $repodir/ecosystem/typescript/sdk
  pnpm install
  pnpm generate-client

  # Typescript SDK docs
  pnpm generate-ts-docs
)

echo
echo "WARNING: If you are adding a new transaction authenticator..."
echo " 1. Check out https://github.com/aptos-labs/aptos-core/blob/main/testsuite/generate-format/README.md"
echo " 2. ecosystem/indexer-grpc/indexer-grpc-fullnode/src/convert.rs must be manually updated"
echo
