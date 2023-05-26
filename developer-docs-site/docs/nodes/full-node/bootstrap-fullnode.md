---
title: "Bootstrap Fullnode from Snapshot"
slug: "bootstrap-fullnode"
sidebar_position: 14
---

# Bootstrap a New Fullnode from Snapshot

This document describes how to bootstrap a new Aptos fullnode quickly using a snapshot. Although you may bootstrap a new fullnode using [state-sync](../../guides/state-sync.md), this might not be an optimal approach after the network has been running for a while; it can either take too much time, or it won't be able to fetch all required data since most nodes have already pruned ledger history. The easiest way to bootstrap a new fullnode is using an existing _fullnode snapshot_. A fullnode snapshot is simply a copy of the storage data of an existing fullnode that can be used to help start other fullnodes more quickly.

:::caution Proceed with caution
It is not recommended to use fullnode snapshots for running fullnodes in production on **mainnet**. This is because snapshots are not fully verified by the fullnode software. As a result, the snapshot may be invalid or contain incorrect data. To prevent security issues, we recommend bootstrapping from snapshot only for test environments, e.g., **devnet** and **testnet**. If you wish to bootstrap from snapshot for **mainnet**, do not use that node in a production environment. Finally, you should always verify that any snapshot you download comes from a reputable source to avoid downloading malicious files.
:::`

## Find an existing fullnode snapshot

There are a number of fullnode snapshots that can be downloaded from different Aptos community members. These include:
- BWareLabs (Testnet and Mainnet): [BWareLabs Aptos Node Snapshots](https://bwarelabs.com/snapshots)
- Polkachu (Mainnet): [Polkachu Aptos Node Snapshots](https://polkachu.com/aptos_snapshots/aptos)

:::tip Questions about snapshot data
Depending on how the snapshot is constructed and compressed, the snapshot files may be different sizes. If you have any questions about the snapshot data, or run into any issues, please reach out to the Aptos community members directly via the [#node-support](https://discord.com/channels/945856774056083548/953421979136962560) channel in [Aptos Discord](https://discord.gg/aptosnetwork).
:::

## Use an existing fullnode snapshot

To use a snapshot, simply download and copy the files to the location of your storage database for the fullnode. This location can be found and updated in the fullnode `yaml` configuration file under `data_dir`. See [Start a public fullnode](fullnode-source-code-or-docker.md) for more information.