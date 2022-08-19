#!/bin/bash

END_INDEX=$(($START_INDEX+$COUNT))
echo "index ranges", $START_INDEX, $END_INDEX

echo "download rosetta-cli"
curl -sSfL https://raw.githubusercontent.com/coinbase/rosetta-cli/master/scripts/install.sh | sh -s

echo "start check:data"
./bin/rosetta-cli --configuration-file crates/aptos-rosetta/rosetta_cli.json check:data --start-block $START_INDEX --end-block $END_INDEX
