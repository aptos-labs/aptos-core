---
title: "Introduction"
slug: "intro"
sidebar_position: 10
---

# Introduction

This tutorial describes how to run Aptos nodes for the Aptos Incentivized Testnet 1 (AIT1) program. It explains the following:

- How to configure a validator node to run in test mode. This will be used during the AIT1 registration stage to validate your eligibility, and 
- How to connect to the incentivized testnet if you are selected to run a validator node.

:::info

For the AIT1, we  recommend that every node operator run both a validator node and a FullNode. Hence, the reference implementation described in these sections will install both the nodes by default. 

:::

## Deploying for Aptos Incentivized Testnet

In order to participate in the incentivized testnet, participants must demonstrate the ability to configure and deploy a node, as well as pass the sanctions screening requirements.

Follow the below steps to participate in the Aptos Incentivized Testnet:
- Follow the instructions to deploy both a validator node and a fullnode in the test mode.
- Navigate to the [Incentivized Testnet registration page](https://community.aptoslabs.com/) and enter information about your node (pub-keys, IP/DNS address).
- If you are selected to run a node, follow instructions in [Connecting to Aptos Incentivized Testnet](connect-to-testnet) to join incentivized testnet.
- Keep the node in healthy state for the entire testing period and follow operational requests as needed.

## Before you proceed

If you are new to Aptos Blockchain, read the following sections before proceeding:

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

We recommend nodes have at least 300GB of disk space to ensure adequate storage space for load testing. You have the option to start with a smaller size and adjust based upon demands. You will be responsible for monitoring your node's disk usage and adjusting appropriately to ensure node uptime.

## Networking configuration requirements

For Validator node:

- Open TCP port 6180, for validators to talk to each other.
- Open TCP port 9101, for getting validator metrics to validate the health stats. (only needed during registration stage)

For Fullnode:

- Open TCP port 6182, for fullnodes to talk to each other.
- Open TCP port 9101, for getting fullnode metrics to validate the health stats. (only needed during registration stage)
- Open TCP port 80/8080, for REST API access.

## Getting started

### Installation
You must run a validator node in test mode to be eligible for incentivized testnet. This is a method we use to verify that a node operator can successfully start a validator node, and have it properly configured with Aptos network identity. 

In test mode, you will be running a local network with one single node, and it should be functioning like a normal blockchain. You can configure an Aptos node in many ways: 

- Using Aptos source code.
- Using Docker, and 
- Using Terraform (for deploying with GCP, AWS and Azure). 

:::tip

For best availability and stability, we recommend that you deploy your node on the cloud. We have provided Terraform support for deploying the node on three cloud providers: GCP, AWS and Azure.

:::

Follow the links below to begin your installation:

* [Using GCP](using-gcp.md)
* [Using AWS](using-aws.md)
* [Using Docker](using-docker.md)
* [Using Aptos source code](using-source-code.md)
