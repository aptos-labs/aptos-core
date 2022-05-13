---
title: "Introduction"
slug: "intro"
sidebar_position: 10
---

# Run a Validator Node

Validator nodes runs a distributed consensus protocol, execute the transaction, and store the transaction and the execution results on the blockchain. Validator nodes decide which transactions will be added to the blockchain and in which order. Read more about [Validator node concepts](/basics/basics-validator-nodes).

For incentivized testnet, we're recommending every node operator to run a validator with a fullnode, so all the reference implementation used here will have both nodes installed by default.

This tutorial explains how to configure a validator node to run on test mode, which will be used during the registration stage to validate your eligibility, and how to connect to incentivized testnet once you're selected to run a validator node. Users not selected to run a validator node can still run a public fullnode to connect to incentivized testnet.

## Before you proceed

Before you get started with this tutorial, read the following sections:

* [Validator node concepts](/basics/basics-validator-nodes).
* [FullNode concepts](/basics/basics-fullnodes).
* [Node networks and synchronization](/basics/basics-node-networks-sync).

## Hardware requirements

We recommend the following hardware resources:

- For running an aptos node on incentivized testnet we recommend the following:

  - **CPU**: 4 cores (Intel Xeon Skylake or newer).
  - **Memory**: 8GiB RAM.

## Storage requirements

The amount of data stored by Aptos depends on the ledger history (length) of the blockchain and the number
of on-chain states (e.g., accounts). These values depend on several factors, including: the age of the blockchain,
the average transaction rate and the configuration of the ledger pruner.

We recommend to bootstrap with 300GB of disk space to leave room for load testing, but if you want to save disk space you can start with smaller size and expand when reaching limit, and you will be responsible for monitoring the disk usage of your node, adjusting it as needed to ensure the node uptime. 

## Networking configuration requirements

For Validator node:

- Open TCP port 6180, for validators to talk to each other.
- Open TCP port 9101, for getting validator metrics to validate the health stats. (only needed during registration stage)

For Fullnode:

- Open TCP port 6182, for fullnodes to talk to each other.
- Open TCP port 9101, for getting fullnode metrics to validate the health stats. (only needed during registration stage)
- Open TCP port 80/8080, for REST API access.

## Getting started
You can configure an Aptos node in many ways: using Aptos source code, using docker, and using Terraform. For best availability and stability, we recommend you to deploy your node on the Cloud. We provided Terraform support for deploying the node on three cloud providers: GCP, AWS and Azure.

In order to participate in the incentivized testnet, participants must demonstrate the ability to configure and deploy a node as well as pass KYC (know your client) sanctions requirements.

High level steps for joining Aptos Incentivized Testnet:
- Follow the instruction to deploy a node (including a validator node and a fullnode) with test mode.
- Navigate to registration page, enter informations about your node (pub-keys, IP/DNS address).
- If you're selected to run a node, follow instructions to join incentivized testnet.
- Keep the node in healthy state for the entire testing period and follow operational requests as needed.

### Installation
Running a validator node in test mode is required to be eligible for incentivized testnet. This is a method we use to verify that a node operator can successfully start a validator node, and have it properly configured with Aptos network identity. In test mode, you will be running a local network with one single node, it should be functioning like a normal blockchain. You can follow those guide to install your node in test mode:

* [Using Aptos source code](run-validator-node-using-source)
* [Using docker](run-validator-node-using-docker)
* [Using GCP](run-validator-node-using-gcp)
* [Using AWS](run-validator-node-using-aws)