---
title: "Bootstrap a New Fullnode"
slug: "bootstrap-fullnode"
sidebar_position: 14
---

# Bootstrap a New Fullnode

Bootstrapping a new fullnode using [state-sync](guides/state-sync) might not be an optimal approach after the network
has been running for a while; it can either take too much time, or it won't be able to fetch the required data since
most nodes have already pruned the ledger history. The easiest way to avoid this is to bootstrap a new fullnode using
an existing _fullnode snapshot_. A fullnode snapshot is simply a copy of the storage data of an existing fullnode that
can be used to help start other fullnodes more quickly.

:::caution Proceed with caution
It is not recommended to use fullnode snapshots for running fullnodes in production on **mainnet**. This is because
snapshots are not fully verified by the fullnode software. As a result, the snapshot may be invalid or contain
incorrect data. To prevent this from causing security issues, we recommend only doing this for test environments,
e.g., **devnet** and **testnet**. If you wish to do this for **mainnet**, do not use it in a production environment.
Also, you should always verify that any snapshot you download comes from a reputable source, to avoid downloading
malicious files.
:::`

## Finding an existing fullnode snapshot

There are a number of fullnode snapshots that can be downloaded from different Aptos community members. These include:
- BWareLabs (Testnet and Mainnet): [BWareLabs Aptos Snapshots](https://bwarelabs.com/snapshots)
- Polkachu (Mainnet): [Polkachu Aptos Node Snapshots](https://polkachu.com/aptos_snapshots/aptos)

:::tip Questions about snapshot data
Depending on how the snapshot is constructed and compressed, the snapshot files may be different sizes. If you have
any questions about the snapshot data, or run into any issues, please reach out to the Aptos community members directly.
:::

## Using an existing fullnode snapshot

To reuse a snapshot, simply download and copy the files to the location of your storage database for the fullnode. This
can be found and/or specified in the fullnode `yaml` configuration file, under `data_dir`. See [Configuring a public
fullnode](nodes/full-node/fullnode-source-code-or-docker#configuring-a-public-fullnode) for more
information.