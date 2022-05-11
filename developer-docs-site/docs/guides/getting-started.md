---
title: "Getting started"
slug: "getting-started"
sidebar_position: 0
---

# Getting Started

This is a guide to kick-start your journey as a developer in the Aptos ecosystem!

## Prepare for Aptos Development

Aptos-core is available on [GitHub](https://github.com/aptos-labs/aptos-core)

```bash
git clone https://github.com/aptos-labs/aptos-core.git
cd aptos-core
./scripts/dev_setup.sh
source ~/.cargo/env
git checkout --track origin/devnet
```

## Explore the tutorials

* [Your first transaction](/tutorials/your-first-transaction)
* [Your first Move module](/tutorials/your-first-move-module)
* [Your first NFT](/tutorials/your-first-nft)
* [Run a local testnet](/tutorials/run-a-local-testnet)
* [Run a FullNode](/tutorials/full-node/run-a-fullnode)

## Start Developing Move Modules

* Learn more about [Aptos Move](/guides/move) (in [presentation format](https://docs.google.com/presentation/d/1MrsumQgdrLnKCaZnrtWvadT5rhOGka-Fhi0OoYtGQo8/edit?usp=sharing))
* Read the [Move book](https://diem.github.io/move/)
* Learn more about [interacting with the Aptos Blockchain](/guides/interacting-with-the-aptos-blockchain)
* Explore the [Framework documentation](https://github.com/aptos-labs/aptos-core/tree/framework-docs)
* Start building and publishing your own modules on our public Devnet or on your own Testnet

## Tools

* [Typescript SDK](https://www.npmjs.com/package/aptos)
* [Wallet extension](https://github.com/aptos-labs/aptos-core/releases/tag/wallet-v0.0.1) and [tutorial](/tutorials/building-wallet-extension)
* [CLI](https://github.com/aptos-labs/aptos-core/releases/tag/aptos-cli-v0.1.0-alpha) and [guide](https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos/README.md)
* While some of us code move in Vi with no syntax highlighting, others also use [VSCode](https://code.visualstudio.com/download) and [this plugin](https://marketplace.visualstudio.com/items?itemName=damirka.move-syntax)

## Devnet Details

* Faucet endpoint: [https://faucet.devnet.aptoslabs.com](https://faucet.devnet.aptoslabs.com)
* REST interface endpoint: [https://fullnode.devnet.aptoslabs.com](https://fullnode.devnet.aptoslabs.com)
* [Genesis](https://devnet.aptoslabs.com/genesis.blob)
* [Waypoint](https://devnet.aptoslabs.com/waypoint.txt)
* [ChainID](https://devnet.aptoslabs.com/chainid.txt)
