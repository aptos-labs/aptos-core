# Aptos CLI Changelog

All notable changes to the Aptos CLI will be captured in this file. This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) and the format set out by [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

# Unreleased

## [7.6.1]
- Mark language version 2.2 as stable.

## [7.6.0]
- Sets up confidential assets for localnet under the experimental address 0x7

## [7.5.0]
- Fix auto-update CLI command to work with more OS's including Mac and Linux on ARM
- Update localnet indexer processors

## [7.4.0]
- UTF-8 characters are now allowed in Move source code comments (and thus error codes).
- Additional compiler warnings on inline functions based on their visibility.
- Various compiler bug fixes.
- Add flag `--file-path <FILE_PATH>...` to `aptos move fmt`, which allows to specify individual files to format.
- Fixed bug where `aptos move fmt` would format current directory if invalid `--package-path` was provided.

## [7.3.0]

- Update boogie from 3.2.4 to 3.5.1.
- Change behavior of `aptos init` to first look and see if the account has an APT balance rather than checking if the account exists

## [7.2.0]

- Add ability to retrieve fungible asset balances
- Add `aptos key extract-public-key` which generates a public key or a proof of possession for the given key.

## [7.1.0]
- Add CLI outputs and on-disk storage to be stored in AIP-80 format.  Will allow for legacy formats to be taken in as well

## [7.0.0]
- Compiler v1 is now deprecated. It is now removed from the Aptos CLI.
- Added a new option `aptos move compile --fail-on-warning` which fails the compilation if any warnings are found.
- We now default to running extended checks when compiling test code (this was previously only done with the option `--check-test-code`, but this is no longer available). However, these checks can be now be skipped with `--skip-checks-on-test-code`.
- Add network to show profiles.
- The new subcommand `aptos update move-mutation-test` will install/update the external binary `move-mutation-test`, which performs mutation testing on a Move project to find blind spots in Move unit tests.
- Add beta simulate command to simulate any transaction from anyone

## [6.2.0]
- Several compiler parsing bugs fixed, including in specifications for receiver style functions
- Remove support for OpenSSL 1.x.x and Ubuntu 20.04, add warning appropriately

## [6.1.1]
- Added a new feature to `aptos workspace run`: The workspace server now listens for a "stop" command from
stdin, which triggers a graceful shutdown when received.

## [6.1.0]
- Remove FFI support from Aptos CLI.
- Various compiler bug fixes.
- Fix for coverage tool crash in the presence of inline functions.

## [6.0.3] - 2025/01/27
- Update the processors used by localnet to 1.26.

## [6.0.2] - 2025/01/24
- Fix `aptos workspace run` so it does not panic when writing to closed stdout/stderr, allowing it to finish its graceful shutdown sequence when used as a child process.

## [6.0.1] - 2025/01/17
- Update Hasura metadata to include `entry_function_contract_address`, `entry_function_module_name`, and `entry_function_function_name` in `user_transactions` table.

## [6.0.0] - 2025/01/14
- Set Compiler v2 as the default compiler and Move 2 as the default language version.
- Add new `--move-1` flag to use Compiler v1 and Move 1.
- Add flag `--benchmark` to `aptos move prove`, which allows to benchmark verification times of individual functions in a package.
- Add flag `--only <name>` to `aptos move prove`, which allows to scope verification to a function.
- Fix `aptos init` to show the explorer link for accounts when account is already created on chain instead of prompting to fund the account.
- Set Compiler v2 as the default compiler and Move 2 as the default language version.
- Add new `--move-1` flag to use Compiler v1 and Move 1.
- Upgrade indexer processors for localnet from 51a34901b40d7f75767ac907b4d2478104d6a515 to 3064a075e1abc06c60363f3f2551cc41f5c091de. Upgrade Hasura metadata accordingly.

## [5.1.0] - 2024/12/13
- More optimizations are now default for compiler v2.
- Downgrade bytecode version to v6 before calling the Revela decompiler, if possible, i.e. no enum types are used. This allows to continue to use Revela until the new decompiler is ready.

## [5.0.0] - 2024/12/11
- [**Breaking Change**] `aptos init` and `aptos account fund-with-faucet` no longer work directly with testnet, you must now use the minting page at the [Aptos dev docs](https://aptos.dev/network/faucet).
## [4.7.0] - 2024/12/10
- [`Fix`] CLI config should not always require a private key field to be present.

## [4.6.0] - 2024/11/29
- Add `--node-api-key` flag to `aptos move replay` to allow for querying the fullnode with an API key.
- Add `--chunk-size` flag to allow configuring chunk size for chunked publish mode.
- Lower the default chunk size for chunked publish mode (`CHUNK_SIZE_IN_BYTES`) from 60,000 to 55,000.

## [4.5.0] - 2024/11/15
- Determine network from URL to make explorer links better for legacy users
- Add support for AIP-80 compliant strings when importing using the CLI arguments or manual input.
- Add option `--print-metadata-only` to `aptos move decompile` and `aptos move disassemble` to print out the metadata attached to the bytecode.
- Add `--existing-hasura-url` flag to localnet to tell it to use an existing Hasura instance instead of run Hasura itself. See https://github.com/aptos-labs/aptos-core/pull/15313.
- Add `--skip-metadata-apply` flag to localnet, in which case we won't try to apply the Hasura metadata.
- Upgrade Hasura image we use from 2.40.2 to 2.44.0.

## [4.4.0] - 2024/11/06
- Fix typos in `aptos move compile` help text.
- Update the default version of `movefmt` to be installed from 1.0.5 to 1.0.6
- Add `--host-postgres-host` flag: https://github.com/aptos-labs/aptos-core/pull/15216.

## [4.3.0] - 2024/10/30
- Allow for setting large-packages module for chunking publish mode with `--large-packages-module-address`
- [`Fix`] Remove unwraps to make outputs go through regular error handling

## [4.2.6] - 2024/10/23
- Fixing issue with `--move-2` flag which was still selecting language version 2.0 instead of 2.1.

## [4.2.5] - 2024/10/23
- Bump to resolve issue with release version inconsistency.

## [4.2.4] - 2024/10/21
- Releasing Move 2.1, which adds compound assignments (`x += 1`) and loop labels to the language. See [Move 2 Release Notes](https://aptos.dev/en/build/smart-contracts/book/move-2).
- multiple bug fixes in the Move 2 compilation chain.
- `aptos move fmt` formats move files inside the `tests` and `examples` directory of a package.
- Added `aptos update prover-dependencies`, which installs the dependency of Move prover, boogie, z3 and cvc5.
- Update the default version of `movefmt` to be installed from 1.0.4 to 1.0.5
- Update the local-testnet logs to use `println` for regular output and reserve `eprintln` for errors.
- Set compiler V2 as default when using `aptos move prove`.

## [4.2.3] - 2024/09/20
- Fix the broken indexer in localnet in 4.2.2, which migrates table info from sycn to async ways.

## [4.2.2] - 2024/09/20
- Fix localnet indexer processors that were emitting spamming logs in 4.2.1.

## [4.2.1] - 2024/09/19
- Fix localnet indexer processors that were failing to startup in 4.2.0

## [4.2.0] - 2024/09/16
- Update latest VM and associated changes
- Update to latest compiler

## [4.1.0] - 2024/08/30
- Marks Move 2 and compiler v2 as stable.
- Adds new `--move-2` flag to work with Move 2 without need for multiple other flags.
- Adds `aptos move lint` to produce lint warnings for the current package. Only a few lint rules are implemented for now,
  but more are coming.
- Adds `aptos move fmt`, which runs the Move formatter, `movefmt`, on the current package. Also adds
  `aptos update movefmt`. This installs / updates the `movefmt` binary.
- Adds safe methods to delete a profile, to rename a profile, and to output the private key of a profile.

## [4.0.0] - 2024/08/13
- **Breaking Change**: change key rotation options such that user has to either pass the name of a new profile or explicitly flag that no profile should be generated, since without this update the interactive profile generator could fail out after the key has already been rotated. This forces the check for new profile validity before doing anything onchain.
- Add support for key rotation to/from Ledger hardware wallets.
- Fixes a bug in the Move Prover leading to internal error in generated boogie (error 'global `#0_info` cannot be accessed')
- **Breaking Change**: A new native function to compute serialized size of a Move value is now supported.

## [3.5.1] - 2024/07/21
- Upgraded indexer processors for localnet from 5244b84fa5ed872e5280dc8df032d744d62ad29d to fa1ce4947f4c2be57529f1c9732529e05a06cb7f. Upgraded Hasura metadata accordingly.
- Upgraded Hasura image from 2.36.1 to 2.40.2-ce. Note that we use the Community Edition, so the console won't ask users to upgrade to enterprise anymore / hint at any enterprise features.
- Fixes a bug in the Move compiler (both v1 and v2) which disallowed `match` as a name for a function or for a variable.

## [3.5.0] - 2024/07/06
- Add balance command to easily get account balances for APT currently
- Add network to config file
- Add explorer links to initialized accounts, and transaction submissions
- Alias some move commands as common misnomers (e.g. build -> compile, deploy -> publish)
- Add "hello_blockchain" template to move init command

## [3.4.1] - 2024/05/31
- Upgraded indexer processors for localnet from ca60e51b53c3be6f9517de7c73d4711e9c1f7236 to 5244b84fa5ed872e5280dc8df032d744d62ad29d. Upgraded Hasura metadata accordingly.

## [3.4.0] - 2024/05/30
- Adds a check for safe usage of randomness features. Public functions are not allowed to call randomness features unless explicitly allowed via attribute `#[lint::allow_unsafe_randomness]`.
- The Move syntax now supports structured attribute names, as in `#[attribute_area::attribute_name]`.
- Upgraded indexer processors for localnet from a11f0b6532349aa6b9a80c9a1d77524f02d8a013 to ca60e51b53c3be6f9517de7c73d4711e9c1f7236. Upgraded Hasura metadata accordingly.

## [3.3.1] - 2024/05/21
- Fixed incompatibility bug that broken local simulation and gas profiling.

## [3.3.0] - 2024/05/03
- **Breaking Change** Update View functions to use BCS for submission.  Allows for all arguments to be supported in view functions.  Note some input arguments that were previously inputted as strings may be handled differently.
- [Early beta release of the Move compiler v2](https://aptos.dev/move/compiler_v2/) is now accessible through the CLI. We now allow specifying the Move compiler version and the Move language version via the CLI.

## [3.2.0] - 2024/03/29
- Renamed `run-local-testnet` to `run-localnet`. `run-local-testnet` is still supported for backwards compatibility.
- Updated localnet node to use latest code changes including long pull

## [3.1.0] - 2024/03/21
- Update `self_update` dependency to support situations where relevant directories (e.g. `/tmp`) exist on different filesystems.
- [bugfix] Rename `--value` back to `--override-size-check` for publishing packages
- Upgraded indexer processors for localnet from cc764f83e26aed1d83ccad0cee3ab579792a0538. This adds support for the `TransactionMetadataProcessor` among other improvements.

## [3.0.2] - 2024/03/12
- Increased `max_connections` for postgres container created as part of localnet to address occasional startup failures due to overloaded DB.

## [3.0.1] - 2024/03/05
- Fix bug in `aptos update revela` if default install directory doesn't exist.

## [3.0.0] - 2024/03/05
- **Breaking Change**: `aptos update` is now `aptos update aptos`.
- Added `aptos update revela`. This installs / updates the `revela` binary, which is needed for the new `aptos move decompile` subcommand.
- Extended `aptos move download` with an option `--bytecode` to also download the bytecode of a module
- Integrated the Revela decompiler which is now available via `aptos move decompile`
- Extended `aptos move disassemble` and the new `aptos move decompile` to also work on entire packages instead of only single files

## [2.5.0] - 2024/02/27
- Updated CLI source compilation to use rust toolchain version 1.75.0 (from 1.74.1).
- Upgraded indexer processors for localnet from 9936ec73cef251fb01fd2c47412e064cad3975c2 to d44b2d209f57872ac593299c34751a5531b51352. Upgraded Hasura metadata accordingly.
- Added support for objects processor in localnet and enabled it by default.

## [2.4.0] - 2024/01/05
- Hide the V2 compiler from input options until the V2 compiler is ready for release
- Updated CLI source compilation to use rust toolchain version 1.74.1 (from 1.72.1).
- Added `for` loop.
  - Syntax: `for (iter in lower_bound..upper_bound) { loop_body }` with integer bounds.
  - Documentation: https://aptos.dev/move/book/loops
- Upgraded indexer processors for localnet from 2d5cb211a89a8705674e9e1e741c841dd899c558 to 4801acae7aea30d7e96bbfbe5ec5b04056dfa4cf. Upgraded Hasura metadata accordingly.
- Upgraded Hasura GraphQL engine image from 2.35.0 to 2.36.1.

## [2.3.2] - 2023/11/28
- Services in the localnet now bind to 127.0.0.1 by default (unless the CLI is running inside a container, which most users should not do) rather than 0.0.0.0. You can override this behavior with the `--bind-to` flag. This fixes an issue preventing the localnet from working on Windows.

## [2.3.1] - 2023/11/07
### Updated
- Updated processor code from https://github.com/aptos-labs/aptos-indexer-processors for the localnet to 2d5cb211a89a8705674e9e1e741c841dd899c558.
- Improved reliability of inter-container networking with localnet.

## [2.3.0] - 2023/10/25
### Added
- Added `--node-api-key`. This lets you set an API key for the purpose of not being ratelimited.

### Updated
- Made the localnet exit more quickly if a service fails to start.
- Updated processor code from https://github.com/aptos-labs/aptos-indexer-processors for the localnet to bcba94c26c8a6372056d2b69ce411c5719f98965.

### Fixed
- Fixed an infrequent bug that caused startup failures for the localnet with `--force-restart` + `--with-indexer-api` by using a Docker volume rather than a bind mount for the postgres storage.
- Fixed an issue where the CLI could not find the Docker socket with some Docker Desktop configurations.

## [2.2.2] - 2023/10/16
### Updated
- Updated processor code from https://github.com/aptos-labs/aptos-indexer-processors for the localnet to d6f55d4baba32960ea7be60878552e73ffbe8b7e.

## [2.2.1] - 2023/10/13
### Fixed
- Fixed postgres data persistence between restarts when using `aptos node run-local-testnet --with-indexer-api`.

## [2.2.0] - 2023/10/11
### Added
- Added `--with-indexer-api` to `aptos node run-local-testnet`. With this flag you can run a full processor + indexer API stack as part of your localnet. You must have Docker installed to use this feature. For more information, see https://aptos.dev/nodes/local-testnet/local-testnet-index.
### Updated
- Updated CLI source compilation to use rust toolchain version 1.72.1 (from 1.71.1).

## [2.1.1] - 2023/09/27
### Added
- Added an option `--print-metadata` to the command `aptos move download` to print out the metadata of the package to be downloaded.
  - Example: `aptos move download  --account 0x1 --package AptosFramework --url https://mainnet.aptoslabs.com/v1 --print-metadata`
### Updated
- The `--with-faucet` flag has been removed from `aptos node run-local-testnet`, we now run a faucet by default. To disable the faucet use the `--no-faucet` flag.
- **Breaking change**: When using `aptos node run-local-testnet` we now expose a transaction stream. Learn more about the transaction stream service here: https://aptos.dev/indexer/txn-stream/. Opt out of this with `--no-txn-stream`. This is marked as a breaking change since the CLI now uses a port (50051 by default) that it didn't used to. If you need this port, you can tell the CLI to use a different port with `--txn-stream-port`.

## [2.1.0] - 2023/08/24
### Updated
- Updated CLI source compilation to use rust toolchain version 1.71.1 (from 1.71.0).
### Added
- Added basic ledger support for CLI
  - Example: `aptos init --ledger` to create a new profile from ledger. After this, you can use it the same way as other profiles.
  - Note: `Ledger Nano s Plus` or `Ledger Nano X` is highly recommended.

## [2.0.3] - 2023/08/04
### Fixed
- Fixed the following input arguments issue when running `aptos move view`
  - #8513: Fixed issue where CLI does not work with big numbers
  - #8982: Fixed args issue when passing in u64/u128/u256 parameters
### Update
- CLI documentation refactor
- Updated CLI source compilation to use rust toolchain version 1.71.0 (from 1.70.0).
### Fixed
* Verify package now does not fail on a mismatched upgrade number

## [2.0.2] - 2023/07/06
### Added
- Added account lookup by authentication key
  - Example: `account lookup-address --auth-key {your_auth_key}`
### Updated
- Updated CLI source compilation to use rust toolchain version 1.70.0 (from 1.66.1).
- Set 2 seconds timeout for telemetry
### Removed
- init command from config subcommand is removed. Please use init from the root command.
  - Example: `aptos config init` -> `aptos init`
### Fixed
- Panic issue when running `aptos move test` is fixed - GitHub issue #8516

## [2.0.1] - 2023/06/05
### Fixed
- Updated txn expiration configuration for the faucet built into the CLI to make localnet startup more reliable.

## [2.0.0] - 2023/06/01
### Added
- Multisig v2 governance support
- JSON input file support
- Builder Pattern support for RestClient
  - NOTE: Methods **new_with_timeout** and **new_with_timeout_and_user_agent** are no longer available.
- Added custom header *x-aptos-client* for analytic purpose

## [1.0.14] - 2023/05/26
- Updated DB bootstrap command with new DB restore features
- Nested vector arg support
    - **Breaking change**: You can no longer pass in a vector like this: `--arg vector<address>:0x1,0x2`, you must do it like this: `--arg 'address:["0x1", "0x2"]'`

## [1.0.13] - 2023/04/27
### Fixed
* Previously `--skip-fetch-latest-git-deps` would not actually do anything when used with `aptos move test`. This has been fixed.
* Fixed the issue of the hello_blockchain example where feature enable was missing

## [1.0.12] - 2023/04/25
### Added
* Support for creating and interacting with multisig accounts v2. More details can be found at [AIP 12](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-12.md).
* Added `disassemble` option to the CLI - This can be invoked using `aptos move disassemble` to disassemble the bytecode and save it to a file
* Fixed handling of `vector<string>` as an entry function argument in `aptos move run`

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
