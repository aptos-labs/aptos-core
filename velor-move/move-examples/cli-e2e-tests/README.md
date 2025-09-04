# CLI E2E tests
These packages, one per production network, are used by the CLI E2E tests to test the correctness of the `velor move` subcommand group. As such there is no particular rhyme or reason to what goes into these, it is meant to be an expressive selection of different, new features we might want to assert.

As it is now the 3 packages share the same source code. Down the line we might want to use these tests to confirm that the CLI works with a new feature as it lands in devnet, then testnet, then mainnet. For that we'd need to separate the source.
