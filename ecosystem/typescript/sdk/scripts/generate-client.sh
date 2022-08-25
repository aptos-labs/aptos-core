#!/usr/bin/env -S bash -xe

shopt -s globstar

cd $(dirname $0)/..

rm -fr src/generated
yarn openapi -i ../../../api/doc/spec.yaml -o ./src/generated -c axios --name AptosGeneratedClient --exportSchemas true

# Add `.js` extension to all relative imports
sed -i 's|from '\''\.\([[:alnum:]_/\.\-\$]*\)*'\'';|from '\''.\1.js'\'';|g' ./src/generated/**/*.ts

# Use proper import syntax for the `axios` default export
sed -i 's/import axios from/import { default as axios } from/g' ./src/generated/**/*.ts
