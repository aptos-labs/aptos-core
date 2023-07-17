---
title: "Node Requirements"
slug: "node-requirements"
---

# Node Requirements

To make your validator node and validator fullnode deployment hassle-free, make sure you have the resources specified in this document. 

## Validator and validator fullnode

- **Both a validator node and a validator fullnode required:** For the Aptos mainnet, we require that you run a validator node and a validator fullnode. We strongly recommend that you run the validator node and the validator fullnode on two separate and independent machines. Make sure that these machines are well-provisioned and isolated from each other. Guaranteeing the resource isolation between the validator and the validator fullnode will help ensure smooth deployment of these nodes.
- **Public fullnode is optional:** We recommend that optionally you run a public fullnode also. However, a public fullnode is not required. If you run public fullnode also, then we strongly recommend that you run the public fullnode on a third machine that is separate and independent from either the validator or the validator fullnode machines. 
:::tip Terraform support
For deploying the nodes in cloud we have provided Terraform support on two cloud providers: **GCP** and **AWS**. See [**Running Validator Node**](running-validator-node/index.md).
:::

- **Open the network ports:** Make sure that you open the network ports prior to connecting to the network. See [Ports](#ports).
- **Close the network ports:** Make sure that you close these ports after either being accepted or rejected for the network.

## Hardware requirements

For running an Aptos **validator node and validator fullnode** we recommend the following hardware resources:

  - **CPU**:
      - 8 cores, 16 threads
      - 2.8GHz, or faster
      - Intel Xeon Skylake or newer
  - **Memory**: 32GB RAM.
  - **Storage**: 2T SSD with at least 40K IOPS and 200MiB/s bandwidth.
  - **Networking bandwidth**: 1Gbps

### Example machine types on various clouds

- **AWS**
    - c6id.8xlarge (if use local SSD)
    - c6i.8xlarge + io1/io2 EBS volume with 40K IOPS.
- **GCP**
    - n2-standard-16 (if use local SSD)
    - n2-standard-32 + pd-ssd with 40K IOPS.

### Motivations for hardware requirements

Hardware requirements depend on the transaction rate and storage demands. The amount of data stored by the Aptos blockchain depends on the ledger history (the number of transactions) of the blockchain and the number of on-chain states (e.g., accounts and resources). Ledger history and the number of on-chain states depend on several factors: the age of the blockchain, the average transaction rate, and the configuration of the ledger pruner.

The current hardware requirements are set considering the estimated growth over the period ending in Q1-2023. Note that we cannot provide a recommendation for archival node storage size as that is an ever-growing number.

**Local SSD vs. network storage**

Cloud deployments require choosing between using local or network storage such as AWS EBS, GCP PD. Local SSD provides lower latency and cost, especially relative to IOPS. 

On the one hand, network storage requires additional CPU support to scale IOPS, but on the other hand, the network storage provides better support for backup snapshots and provide resilience for the nodes in scenarios where the instance is stopped. Network storage makes it easier to support storage needs for high availability.

## Ports

When you are running a validator node, you are required to open network ports on your node to allow other nodes to connect to you. For fullnodes this is optional.

### Network types

Your node can be configured so that each of these networks can connect using a different port on your node.

There are three types of Aptos networks:
1. **Validator network:** A validator node connects to this network.
2. **Public network:** A public fullnode connects to this network.
3. **Validator fullnode network:** A validator fullnode (VFN) connects to this network. The VFN network allows the validator fullnode to connect to a specific validator.

You can configure the port settings on your node using the configuration YAML file. See the [example configuration YAML here](https://github.com/aptos-labs/aptos-core/blob/4ce85456853c7b19b0a751fb645abd2971cc4c0c/docker/compose/aptos-node/fullnode.yaml#L10-L9). With this configuration YAML on your node, the public network connects to your node on port 6182 and the VFN network on 6181. Because these port settings are configurable, we don't explicitly say port X is for network Y.

### Port settings

:::tip Default port settings
The recommendations described below assume the default port settings used by validators, validator fullnodes and public fullnodes. **We recommend that you do not expose any other ports while operating a node.** If you have changed the default port settings, then you should adjust the recommendations accordingly.
:::

#### For the validator:

- Open the following TCP ports:
  - `6180` – Open publicly to enable the validator to connect to other validators in the network.
  - `6181` – Open privately to only be accessible by your validator fullnode.
- Close the following TCP ports:
  - `6182` – To prevent public fullnode connections
  - `9101` – To prevent unauthorized metric inspection
  - `80/8080` – To prevent unauthorized REST API access

#### For the validator fullnode:

- Open the following TCP ports:
  - `6182` – Open publicly to enable public fullnodes to connect to your validator fullnode.
  - `6181` – Open privately to only be accessible by your validator.
- Close the following TCP ports:
  - `9101` – To prevent unauthorized metric inspection
  - `80/8080` – To prevent unauthorized REST API access

#### For a public fullnode:
- Open the TCP port `6182` publicly to enable other public fullnodes to connect to your node. 
- Close the following TCP ports:
  - `9101` – To prevent unauthorized metric inspection
  - `80/8080` – To prevent unauthorized REST API access

:::caution Exposing services
We note that the inspection port (`9101`) and the REST API port (`80` or `8080`) are likely useful for your internal network, e.g., application development and debugging. However, the inspection port should never be exposed publicly as it can be easily abused. Similarly, if you choose to expose the REST API endpoint publicly, you should deploy an additional authentication or rate-limiting mechanism to prevent abuse.
:::
