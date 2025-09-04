# Validator FullNode (VFN) NHC periodic checker

## Description
This tool is a client to NHC that does the following:
1. Get the validator set from any node participating in a network we want to test.
2. Process that to create a map from operator account address to VFN network addresses.
3. Query NHC for each VFN.
4. Push the results to BigQuery.

The original intent behind this tool is to confirm operators are running quality, operational VFNs as part of AIT3. This tool can be easily adapted for other use cases down the line.

## Local development
Run a local network with both a validator and VFNs:
```
cargo run -p velor-forge-cli -- --suite "run_forever" --num-validators 4 --num-validator-fullnodes 2 --mempool-backlog 5000 test local-swarm
```

Run local NHC:
```
cargo run -p velor-node-checker -- server run --baseline-node-config-paths ~/a/internal-ops/infra/apps/node-checker/configs/ait3_vfn.yaml --listen-address 0.0.0.0
```

Run the tool:
```
cargo run -p velor-fn-check-client -- --nhc-address http://127.0.0.1:20121 --nhc-baseline-config-name ait3_vfn --big-query-key-path ~/a/internal-ops/helm/observability-center/files/bigquery-cron-key.json --big-query-dry-run check-validator-full-nodes --node-address http://127.0.0.1:8080
```
Output: https://gist.github.com/banool/29e18a863709c1998891551da1dfb429.

Submitting to BigQuery instead:
```
cargo run -p velor-fn-check-client -- --nhc-address http://127.0.0.1:20121 --nhc-baseline-config-name ait3_vfn --big-query-key-path ~/a/internal-ops/helm/observability-center/files/bigquery-cron-key.json --big-query-dry-run check-public-full-nodes --input-file ~/test_data.json
```

## Deployment
This is automatically built into the tools image.

## Helpful stuff
This command will show you the validator set on chain using the CLI:
```
velor node show-validator-set --url https://fullnode.devnet.velorlabs.com
```
