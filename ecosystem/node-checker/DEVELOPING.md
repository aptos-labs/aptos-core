# Developing NHC
To develop NHC, you should first run two nodes of the same type. See [this wiki](https://aptos.dev/nodes/full-node/fullnode-for-devnet) for guidance on how to do this. You may also target a known existing FullNode with its metrics port open.

The below command assumes we have a fullnode running locally, the target node (the node under investigation), and another running on a machine in our network, the baseline node (the node we compare the target to):
```
cargo run -- --baseline-node-url 'http://192.168.86.2' --target-node-url http://localhost --evaluators state_sync_version --allow-preconfigured-test-node-only
```
This runs NHC in sidecar mode, where only the `/check_preconfigured_node` endpoint can be called, which will target the node running on localhost.

Once the service is running, you can query it like this:
```
$ curl -s localhost:20121/check_preconfigured_node | jq .
{
  "evaluations": [
    {
      "headline": "State sync version is within tolerance",
      "score": 100,
      "explanation": "Successfully pulled metrics from target node twice, saw the version was progressing, and saw that it is within tolerance of the baseline node. Target version: 1882004. Baseline version: 549003. Tolerance: 1000"
    }
  ],
  "summary_score": 100,
  "summary_explanation": "100: Awesome!"
}
```
