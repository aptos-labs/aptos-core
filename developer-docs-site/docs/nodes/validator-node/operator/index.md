---
title: "Operator"
slug: "index"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Operator

If you are an operator participating in the Aptos network, then use this document to perform the operator tasks such as deploying a validator node and validator fullnode, registering the nodes on the Aptos community platform, and performing the validation. 

:::tip Both validator node and validator fullnode are required for mainnet
For participating in the Aptos mainnet, you must deploy both a validator node and a validator fullnode. 
:::

## Deploy the nodes and register

### Summary steps

<center>
<ThemedImage
alt="Operator Flow"
sources={{
    light: useBaseUrl('/img/docs/operator-flow.svg'),
    dark: useBaseUrl('/img/docs/operator-flow-dark.svg'),
  }}
/>
</center>

### Detailed steps

:::tip Petra on Chrome browser only
The [Petra wallet](/docs/guides/install-petra-wallet.md) is supported only on the Chrome browser. You can also use Petra extension on [Brave browser](https://brave.com/) and [Kiwi browser](https://kiwibrowser.com/) and [Microsoft Edge browser](https://www.microsoft.com/en-us/edge).
:::

**Step 1:** Before you proceed, read the [**Node Requirements**](/docs/nodes/validator-node/operator/node-requirements.md) and make sure that your hardware, storage and network resources satisfy the node requirements.

**Step 2:** **Deploy the nodes**. Follow the detailed node installation steps provided in [**Running Validator Node**](running-validator-node/index.md) and deploy a validator node and a validator fullnode in the test mode.

:::tip Set your nodes in test mode
**Make sure to set your nodes in the Test mode.** Instructions are provided in the node installation sections. Test mode is required for Aptos Labs to do a health check on your nodes.
:::

**Step 3:** Go to the [Aptos Community page](https://aptoslabs.com/community) and register your node by clicking on **NODE REGISTRATION** button.

Provide the following details of your validator node and validator fullnode on this node registration screen. All the public key information you need is in the `~/$WORKSPACE/keys/public-keys.yaml` file (under any circumstances do not enter anything from private keys). For more on `WORKSPACE` see the node installation guide you used to deploy the node.

  - **CONSENSUS KEY**: Value of `consensus_public_key` from `public-keys.yaml` file.
  - **CONSENSUS POP**: Value of `consensus_proof_of_possession` from `public-keys.yaml` file.
  - **ACCOUNT KEY**: Value of `account_public_key` from `public-keys.yaml` file.
  - **VALIDATOR NETWORK KEY**: Value of `validator_network_public_key` from `public-keys.yaml` file.

**Step 4:** Next, click on **VALIDATE NODE** on the community page. If your nodes pass healthcheck, you will be prompted to complete the identity verification process.

  The Aptos team will perform a node health check on your validator and validator fullnode, using the [Node Health Checker](/nodes/node-health-checker/index). When Aptos confirms that your nodes are healthy, you will be asked to complete the KYC process.

**Step 5:** When you receive notification from Aptos that you are selected, then proceed to **Connect to Aptos network** step.

## Connect to Aptos network

See [Connecting to Aptos Network](/nodes/validator-node/operator/connect-to-aptos-network) for detailed steps.
