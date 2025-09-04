# Testsuites

This directory contains a mix of test utilities written in both Python and Rust
* Forge - a unified cluster-testing framework for orchestrating both local swarm and large-scale kubernetes based Velor workloads
* Forge Wrapper - a set of Python utilities to schedule and manage Forge jobs on supported kubernetes clusters
* Testcases - Forge test cases
* Verify - Python utilities used to invoke replay-verify and module-verify workflows

## Debugging
If you run into an issue like this:
```
no match for platform in manifest: not found
```

It is likely because you're on an ARM machine. Try running your command with `DOCKER_DEFAULT_PLATFORM=linux/amd64`, e.g.
```
DOCKER_DEFAULT_PLATFORM=linux/amd64 poetry run python indexer_grpc_local.py start
```

