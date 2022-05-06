---
title: "Getting started for the Hackathon"
slug: "getting-started-for-hackathon"
sidebar_position: 0
---

# Getting Started

Today is a momentous occasion! You probably came to this page because you're attending our first hackathon ever!!!

Please reach out to James to gain access to our hackathon Discord!

Please find a team, meet new people, talk with the team, have fun, and hack!

The rest of this is a guide to kick-start your journey as a hacker in the Aptos Ecosystem!

## Prepare for Aptos Development

Aptos-core is available on [GitHub](https://github.com/aptos-labs/aptos-core)

```bash
git clone https://github.com/aptos-labs/aptos-core.git
cd aptos-core
./scripts/dev_setup.sh
source ~/.cargo/env
git checkout --track origin/hackathon
```

Note: we will be using the hackathon branch as it contains some really cool new features!

## Explore the tutorials

* [Slides from today's Move presentation](https://docs.google.com/presentation/d/1MrsumQgdrLnKCaZnrtWvadT5rhOGka-Fhi0OoYtGQo8/edit?usp=sharing)
* [Your first transaction](/tutorials/your-first-transaction)
* [Your first Move module](/tutorials/your-first-move-module)
* [Your first NFT](/tutorials/your-first-nft)
* [Run a local testnet](/tutorials/run-a-local-testnet)
* [Run a FullNode](/tutorials/full-node/run-a-fullnode)

## Aptos Tooling

* [Releases for the hackathon](https://github.com/aptos-labs/aptos-core/releases/)
* While some of us code move in Vi with no syntax highlighting, others also use [VSCode](https://code.visualstudio.com/download) and [this plugin](https://marketplace.visualstudio.com/items?itemName=damirka.move-syntax)
* We have a [Wallet with some basic DApp functionality](/tutorials/building-wallet-extension)
* We have a [CLI](https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos/README.md) that makes it easy to develop and deploy Move modules
* There's also a [Typescript SDK](https://www.npmjs.com/package/aptos)

## Start Developing Move Modules

* Read the [Move book](https://diem.github.io/move/)
* Learn more about [interacting with the Aptos Blockchain](/guides/interacting-with-the-aptos-blockchain)
* Explore the [Framework documentation](https://github.com/aptos-labs/aptos-core/tree/framework-docs)
* Start building and publishing your own modules on our public Devnet or on your own Testnet

## Devnet Details

* Faucet endpoint: [https://faucet.devnet.aptoslabs.com](https://faucet.devnet.aptoslabs.com)
* REST interface endpoint: [https://fullnode.devnet.aptoslabs.com](https://fullnode.devnet.aptoslabs.com)
* [Genesis](https://devnet.aptoslabs.com/genesis.blob)
* [Waypoint](https://devnet.aptoslabs.com/waypoint.txt)
* [ChainID](https://devnet.aptoslabs.com/chainid.txt)
