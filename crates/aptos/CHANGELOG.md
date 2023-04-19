# Aptos CLI Changelog

All notable changes to the Aptos CLI will be captured in this file. This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) and the format set out by [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [1.0.12]
### Added
* Support for creating and interacting with multisig accounts v2. More details can be found at [AIP 12](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-12.md).

## [1.0.11] - 2023/04/14
### Fixed
* Fixed creating a new test account with `aptos init` would fail if the account didn't already exist

## [1.0.10] - 2023/04/13
### Fixed
* If `aptos init` is run with a faucet URL specified (which happens by default when using the local, devnet, or testnet network options) and funding the account fails, the account creation is considered a failure and nothing is persisted. Previously it would report success despite the account not being created on chain.
* When specifying a profile where the `AuthenticationKey` has been rotated, now the `AccountAddress` is properly used from the config file
* Update `aptos init` to fix an incorrect account address issue, when trying to init with a rotated private key. Right now it does an actual account lookup instead of deriving from public key

### Added
* Updates to prover and framework specs

## [1.0.9] - 2023/03/29
### Added
* `aptos move show abi` allows for viewing the ABI of a compiled move package
* Experimental gas profiler with the `--profile-gas` flag on any transaction submitting CLI command
* Updates to the prover and framework specs

## [1.0.8] - 2023/03/16
### Added
* Added an `aptos account derive-resource-account-address` command to add the ability to derive an address easily
* Added the ability for different input resource account seeds, to allow matching directly with onchain code
* Added beta support for coverage via `aptos move coverage` and `aptos move test --coverage`
* Added beta support for compiling with bytecode dependencies rather than source dependencies

### Fixed
* All resource account commands can now use `string_seed` which will match the onchain representation of `b"string"` rather than always derive a different address
* Tests that go over the bytecode size limit can now compile
* `vector<string>` inputs to now work for both `aptos move view` and `aptos move run`
* Governance proposal listing will now not crash on the latest on-chain format
* Move compiler will no longer use an environment variable to communicate between compiler and CLI for the bytecode version

## [1.0.7]
* For logs earlier than 1.0.7, please check out the [releases on GitHub](https://github.com/aptos-labs/aptos-core/releases?q="Aptos+CLI+Release")
