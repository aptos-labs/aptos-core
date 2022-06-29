---
title: "Node Requirements"
slug: "node-requirements"
---

# Node Requirements

Follow the requirements specified in this document to make your AIT-2 Validator and FullNode deployment hassle-free. 

## Validator and FullNode

- For the AIT-2, we require that you run the Validator. We recommend that optionally you run a FullNode also. However, a FullNode is not required. 
- If you run FullNode also, then we strongly recommend that you run the Validator and the FullNode on two separate and independent machines. Make sure that these machines are well-provisioned and isolated from each other. Guaranteeing the resource isolation between the Validator and the FullNode will help ensure smooth deployment of these nodes.
- For best availability and stability, **we recommend that you deploy your node on the cloud**. We have provided Terraform support for deploying the node on three cloud providers: GCP, AWS and Azure. See [Validators](/nodes/validator-node/validators).
- Make sure that you open the network ports prior to July 12, before the AIT-2 goes live. See [Networking configuration requirements](#networking-configuration-requirements).
- Make sure that you close these ports after either being accepted or rejected for the AIT-2.

## Validator node in test mode 

You must run a Validator node in the test mode to be eligible for AIT-2. This is a method Aptos Labs uses to verify that a node operator can successfully start a Validator node and configure it properly with the Aptos network identity. 

In test mode, you will be running a local network with one single node, and it should be functioning like a normal blockchain.

## Hardware requirements

For running an Aptos node we recommend the following hardware resources:

  - **CPU**: 4 cores (Intel Xeon Skylake or newer).
  - **Memory**: 8GiB RAM.

## Storage requirements

The amount of data stored by the Aptos Blockhain depends on the ledger history (length) of the blockchain and the number of on-chain states (e.g., accounts). These values depend on several factors, including: the age of the blockchain, the average transaction rate and the configuration of the ledger pruner.

We recommend nodes have at least 300GB of disk space to ensure adequate storage space for load testing. You have the option to start with a smaller size and adjust based upon demands. You will be responsible for monitoring your node's disk usage and adjusting appropriately to ensure node uptime.

## Networking configuration requirements

For the Validator:

- Open the TCP port 6180, for the Validators to talk to each other.
- Open the TCP port 9101, for getting the Validator metrics to validate the health stats (only needed during registration stage).

For the Fullnode:

- Open the TCP port 6182, for Fullnodes to talk to each other.
- Open the TCP port 9101, for getting the Fullnode metrics to validate the health stats (only needed during registration stage).
- Open the TCP port 80/8080, for the REST API access.

