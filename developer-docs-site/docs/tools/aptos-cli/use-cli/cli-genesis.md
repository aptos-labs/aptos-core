---
title: "Genesis"
id: "cli-genesis"
---

## Genesis ceremonies

The `aptos` tool supports bootstrapping new blockchains through what is known as a genesis ceremony. The output of the genesis ceremony is the output of move instructions that prepares a blockchain for online operation. The input consists of:

- A set of validators and their configuration
- The initial set of Move modules, known as a framework
- A unique `ChainId` (u8) that distinguishes this from other deployments
- For test chains, there also exists an account that manages the minting of AptosCoin

## Generating genesis

- The genesis organizer constructs a `Layout` and distributes it.
- The genesis organizer prepares the Aptos framework's bytecode and distributes it.
- Each participant generates their `ValidatorConfiguration` and distributes it.
- Each participant generates a `genesis.blob` from the resulting contributions.
- The genesis organizer executes the `genesis.blob` to derive the initial waypoint and distributes it.
- Each participant begins their `aptos-node`. The `aptos-node` verifies upon startup that the `genesis.blob` with the waypoint provided by the genesis organizer.
- The blockchain will begin consensus after a quorum of stake is available.

### Prepare aptos-core

The following sections rely on tools from the Aptos source. See [Building Aptos From Source](../../../guides/building-from-source.md) for setup.

### The `layout` file

The layout file contains:

- `root_key`: an Ed25519 public key for AptosCoin management.
- `users`: the set of participants
- `chain_id`: the `ChainId` or a unique integer that distinguishes this deployment from other Aptos networks

An example:

```
root_key: "0xca3579457555c80fc7bb39964eb298c414fd60f81a2f8eedb0244ec07a26e575"
users:
  - alice
  - bob
chain_id: 8
```

### Building the Aptos Framework

From your Aptos-core repository, build the framework and package it:

```
cargo run --package framework
mkdir aptos-framework-release
cp aptos-framework/releases/artifacts/current/build/**/bytecode_modules/* aptos-framework-release
```

The framework will be stored within the `aptos-framework-release` directory.

### The `ValidatorConfiguration` file

The `ValidatorConfiguration` file contains:

- `account_address`: The account that manages this validator. This must be derived from the `account_key` provided within the `ValidatorConfiguration` file.
- `consensus_key`: The public key for authenticating consensus messages from the validator
- `account_key`: The public key for the account that manages this validator. This is used to derive the `account_address`.
- `network_key`: The public key for both validator and fullnode network authentication and encryption.
- `validator_host`: The network address where the validator resides. This contains a `host` and `port` field. The `host` should either be a DNS name or an IP address. Currently only IPv4 is supported.
- `full_node_host`: An optional network address where the fullnode resides. This contains a `host` and `port` field. The `host` should either be a DNS name or an IP address. Currently only IPv4 is supported.
- `stake_amount`: The number of coins being staked by this node. This is expected to be `1`, if it is different the configuration will be considered invalid.

An example:

```
account_address: ccd49f3ea764365ac21e99f029ca63a9b0fbfab1c8d8d5482900e4fa32c5448a
consensus_key: "0xa05b8f41057ac72f9ca99f5e3b1b787930f03ba5e448661f2a1fac98371775ee"
account_key: "0x3d15ab64c8b14c9aab95287fd0eb894aad0b4bd929a5581bcc8225b5688f053b"
network_key: "0x43ce1a4ac031b98bb1ee4a5cd72a4cca0fd72933d64b22cef4f1a61895c2e544"
validator_host:
  host: bobs_host
  port: 6180
full_node_host:
  host: bobs_host
  port: 6182
stake_amount: 1
```

To generate this using the `aptos` CLI:

1. Generate your validator's keys:

```
cargo run --package aptos -- genesis generate-keys --output-dir bobs
```

2. Generate your `ValidatorConfiguration`:

```
cargo run --package aptos -- \\
    genesis set-validator-configuration \\
    --keys-dir bobs \\
    --username bob \\
    --validator-host bobs_host:6180 \\
    --full-node-host bobs_host:6180 \\
    --local-repository-dir .
```

3. The last command will produce a `bob.yaml` file that should be distributed to other participants for `genesis.blob` generation.

### Generating a genesis and waypoint

`genesis.blob` and the waypoint can be generated after obtaining the `layout` file, each of the individual `ValidatorConfiguration` files, and the framework release. It is important to validate that the `ValidatorConfiguration` provided in the earlier stage is the same as in the distribution for generating the `genesis.blob`. If there is a mismatch, inform all participants.

To generate the `genesis.blob` and waypoint:

- Place the `layout` file in a directory, e.g., `genesis`.
- Place all the `ValidatorConfiguration` files into the `genesis` directory.
- Ensure that the `ValidatorConfiguration` files are listed under the set of `users` within the `layout` file.
- Make a `framework` directory within the `genesiss` directory and place the framework release `.mv` files into the `framework` directory.
- Use the `aptos` CLI to generate genesis and waypoint:

```
cargo run --package aptos -- genesis generate-genesis --local-repository-dir genesis
```

### Starting an `aptos-node`

Upon generating the `genesis.blob` and waypoint, place them into your validator and fullnode's configuration directory and begin your validator and fullnode.
