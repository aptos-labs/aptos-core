---
title: "Node Requirements"
slug: "node-requirements"
---

# Node Requirements

Follow the requirements specified in this document to make your Testnet/Mainnet Validator and fullnode deployment hassle-free.

## Validator and fullnode

- For the Testnet/Mainnet, we require that you run the Validator. We recommend that optionally you run a fullnode also. However, a fullnode is not required. 
- If you run fullnode also, then we strongly recommend that you run the Validator and the fullnode on two separate and independent machines. Make sure that these machines are well-provisioned and isolated from each other. Guaranteeing the resource isolation between the Validator and the fullnode will help ensure smooth deployment of these nodes.
- For best availability and stability, **we recommend that you deploy your node on the cloud**. We have provided Terraform support for deploying the node on three cloud providers: GCP, AWS and Azure. See [Validators](/nodes/validator-node/validators).
- Make sure that you open the network ports prior to the date the AIT goes live. See [Networking configuration requirements](#networking-requirements).
- Make sure that you close these ports after either being accepted or rejected for the AIT.

## Validator node in test mode

You must run a validator node in the test mode to be eligible for Testnet/Mainnet. This is a method Aptos Labs uses to verify that a node operator can successfully start a validator node and configure it properly with the Aptos network identity. 

In test mode, you will be running a local network with one single node, and it should be functioning like a normal blockchain.

## Hardware requirements

For running an Aptos **validator and fullnode** we recommend the following hardware resources:

  - **CPU**:
      - 8 cores, 16 threads
      - 2.8GHz, or faster
      - Intel Xeon Skylake or newer
  - **Memory**: 32GB RAM.
  - **Storage**: 1T SSD with at least 40K IOPS and 200MiB/s bandwidth.
  - **Networking bandwidth**: 1Gbps

Example machine types on various clouds:
  - AWS
      - c6id.4xlarge (if use local SSD)
      - c6i.8xlarge + io1/io2 EBS volume with 40K IOPS.
  - GCP
      - n2-standard-16 (if use local SSD)
      - n2-standard-32 + pd-ssd with 40K IOPS.

### Implications on hardware requirements

The amount of data stored by the Aptos Blockhain depends on the ledger history (the number of transactions) of the blockchain and the number of on-chain states (e.g., accounts and resources). These values depend on several factors, including: the age of the blockchain, the average transaction rate, and the configuration of the ledger pruner.

Hardware requirements depend on the transaction rate and storage demands. Over time, hardware requirements will need to scale with these demands. The current hardware requirements are set with the consideration of estimated growth over the next 6 months.

**Local SSD v.s. network storage**

Cloud deployments typically must make a decision between using local or network storage (e.g., AWS EBS, GCP PD). Local SSD typically provides lower latency and cost especially relative to IOPS. Network storage usually requires additional CPU support to scale IOPS. In contrast, network storage provides better support for backup/snapshot support and resilience for nodes in scenarios where the instance is stopped. Network storage makes it easier to support storage for high availability.

## Ports

When you are running a validator node, you are required to open network ports on your node to allow other nodes to connect to you. For fullnodes this is optional.

There are three types of Aptos networks. Your node can be configured so that each of these networks can connect to your node using a different port on your node.

1. The validator network: A validator node connects to this network.
2. The public network. A public fullnode connects to this network.
3. The validator fullnode network (VFN network): A validator fullnode connects to this network. The VFN network allows the validator fullnode to connect to the specific validator.

You can configure the port settings on your node using the configuration YAML file. See the [example configuration YAML here](https://github.com/aptos-labs/aptos-core/blob/4ce85456853c7b19b0a751fb645abd2971cc4c0c/docker/compose/aptos-node/fullnode.yaml#L10-L9). With this configuration YAML on your node, the public network connects to your node on port 6182 and the VFN network on 6181. Because these port settings are configurable, we don't explicitly say port X is for network Y.

### Port settings

For the Validator:

- Open the TCP port 6180, for the Validators to talk to each other.
- Open the TCP port 9101, for getting the Validator metrics to validate the health stats (only needed during registration stage).

For the fullnode:

- Open the TCP port 6182, for fullnodes to talk to each other.
- Open the TCP port 9101, for getting the fullnode metrics to validate the health stats (only needed during registration stage).
- Open the TCP port 80/8080, for the REST API access.

