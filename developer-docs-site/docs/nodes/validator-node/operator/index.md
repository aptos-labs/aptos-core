---
title: "Operator"
slug: "index"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Operator (Operator actions only - DRAFT)

***This is the landing page for the operator persona. I will edit this page. For now, only a placeholder.***

See below the summary flowcharts and detailed steps you will execute as operator.

:::caution Chrome browser only
The new Petra (Aptos Wallet) is supported only on the Chrome browser. 
:::

## Sign-in and connect Wallet

REMOVE THIS.
### Summary steps

## Deploy the validator node and register the node

USE THIS BY EDITING.  

### Summary steps

<center>
<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/install-validator-and-register.svg'),
    dark: useBaseUrl('/img/docs/install-validator-and-register-dark.svg'),
  }}
/>
</center>

### Detailed steps

1. Read the Node Requirements.

  :::tip

  Before you proceed, make sure that your hardware, storage and network resources satisfy the [Node Requirements](/docs/nodes/validator-node/operator/node-requirements.md).
  :::

2. Follow the detailed node installation steps provided in [Running Validator Node](running-validator-node/index.md) and deploy a validator node in the test mode.

  **Make sure to set your node in the Test mode.** Instructions are provided in the node installation sections. Test mode is required for Aptos Labs to do a health check on your node.

3. Come back to the Aptos Community page and register your node by clicking on Step 4: **NODE REGISTRATION** button.

  Provide the details of your validator node on this node registration screen, all the public key information you need is in the `~/$WORKSPACE/keys/public-keys.yaml` file (please don't enter anything from private keys).

    - OWNER KEY: the first wallet public key. From Settings -> Credentials
    - CONSENSUS KEY: consensus_public_key from `public-keys.yaml`
    - CONSENSUS POP: consensus_proof_of_possession from `public-keys.yaml`
    - ACCOUNT KEY: account_public_key from `public-keys.yaml`
    - VALIDATOR NETWORK KEY: validator_network_public_key from `public-keys.yaml`

4. Next, click on **VALIDATE NODE**. If your node passes healthcheck, you will be prompted to complete the identity verification process.

  The Aptos team will perform a node health check on your validator, using the [Node Health Checker](/nodes/node-health-checker/index). When Aptos confirms that your node is healthy, you will be asked to complete the KYC process.

5. Wait for the selection announcement. If you are selected, the Aptos team will airdrop coins into your owner wallet address. If you do not see airdropped coins in your owner wallet, you were not selected.

6. If you are selected, then proceed to **Iniatilize staking pool** step.

## Initialize staking pool

REMOVE THIS.

## Connect to AIT-3 and join the validator set

### Detailed steps

See [Connecting to Aptos Network](/nodes/validator-node/operator/connect-to-aptos-network) for detailed steps.

## Vote

REMOVE THIS.
