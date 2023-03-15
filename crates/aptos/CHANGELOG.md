# Aptos CLI Changelog

All notable changes to the Aptos Node SDK will be captured in this file. This changelog is written by hand for now. It adheres to the format set out by [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

**Note:** The Aptos TS SDK does not follow semantic version while we are in active development. Instead, breaking changes will be announced with each devnet cut. Once we launch our mainnet, the SDK will follow semantic versioning closely.

## Unreleased
### Features
* Added a `aptos account derive-resource-account-address` command to add the ability to derive an address easily
* Added the ability for different input resource account seeds, to allow matching directly with onchain code
* Added beta support for coverage via `aptos move coverage` and `aptos move test --coverage`
* Added beta support for compiling with bytecode dependencies rather than source dependencies

### Bug fixes
* All resource account commands can now use `string_seed` which will match the onchain representation of `b"string"` rather than always derive a different address
* Tests that go over the bytecode size limit can now compile
* `vector<string>` inputs to now work for both `aptos move view` and `aptos move run`
* Governance proposal listing will now not crash on the latest on-chain format
* Move compiler will no longer use an environment variable to communicate between compiler and CLI for the bytecode version

### Known issues
* `vector<string>` inputs don't support commas in the string inputs


## 1.0.7
* For logs earlier than 1.0.7, please check out the [releases on GitHub](https://github.com/aptos-labs/aptos-core/releases?q="Aptos+CLI+Release")