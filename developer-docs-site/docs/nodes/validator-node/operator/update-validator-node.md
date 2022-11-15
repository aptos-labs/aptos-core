---
title: "Update Aptos Validator Node"
slug: "update-validator-node"
---

# Update Aptos Validator Node via Failover

TODO: Ask Jing how to notify us of HW spec changes.

You will likely have to upgrade or replace your validator node (VN) at some point, such as for maintenance or outages. Start anew by [creating a new validator fullnode (VFN)](running-validator-node/index.md). To minimize downtime, we recommend you then convert your live validator fullnode to your validator node, and backfill the validator fullnode.

Since you are already running [a validator node and a validator fullnode](node-requirements.md), you have at your fingertips the means to replace your validator node immediately. Simply convert your validator fullnode to a validator node and then backfill the validator fullnode with either the updated validator node or an entirely new validator fullnode.

This page explains how to make this swap, which largely amounts to switching out files and configuration settings between the two nodes.

## Prepare

First, understand the data is almost identical between the two nodes. The VFN is missing the `consensus_db` and `secure-data.json`, but it is otherwise largely ready for conversion into a validator node. See the files in the [validator setup](running-validator-node/index.md) documentation you used for the full list.

To failover from an outdated or erroneous validator node to an updated and reliable validator fullnode, follow these steps:

1. Ensure your machine meets the [validator hardware requirements](node-requirements.md#hardware-requirements).
1. Update your validator fullnode with the latest version of the:
   * [required packages Aptos depends upon](../../../guides/getting-started#prepare-development-environment)
   * [Aptos CLI](../../../cli-tools/aptos-cli-tool/install-aptos-cli.md)

(Doc files list in docs and structure from forum on index above)


