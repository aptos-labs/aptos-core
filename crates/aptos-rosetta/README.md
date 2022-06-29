# Aptos Rosetta Implementation
This implementation is built for running a local proxy against
a local full node.  However, for testing purposes, this can be used
against an external REST endpoint.

## CLI testing
The [Rosetta CLI](https://www.rosetta-api.org/docs/rosetta_cli.html) can be run with the [rosetta_cli.json](./rosetta_cli.json)
file to run the automated checks.  Additionally, the [aptos.ros](./aptos.ros)
file uses the Rosetta CLI DSL to describe the possible operations that
can be run.

## Future work
Currently, this only supports P2P transactions, we may support more
types of transactions in the future.
