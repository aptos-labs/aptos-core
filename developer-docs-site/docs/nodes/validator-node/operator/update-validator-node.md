---
title: "Update Aptos Validator Node"
slug: "update-validator-node"
---

# Update Aptos Validator Node via Failover

TODO: Ask Jing how to notify us of HW spec changes.

You will likely have to upgrade or replace your validator node at some point, such as for maintenance or outages. You may start anew by [creating a new validator fullnode](running-validator-node/index.md). Or to minimize downtime, we recommend you convert your validator fullnode to a validator node.

Since you are already running [a validator node and a validator fullnode](node-requirements.md), you have at your fingertips the means to quickly replace your validator node on the fly. Simply convert your validator fullnode to a validator node and backfill the validator fullnode with either the updated validator node or an entirely new validator fullnode.

This page explains how to make this swap using Docker, yet the instructions can be readily adapted to your own validator node setup.

(Doc files list in docs and structure from forum on index above)


