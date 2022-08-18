---
title: "Steps in AIT-3"
slug: "steps-in-ait3"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Steps in AIT-3

:::caution DRAFT-only
These AIT-3 docs are draft-only for now.
:::

See below the summary flowcharts and detailed steps you will execute while participating.

## Install wallet, deploy the validator node and register 

Participants in the AIT-3 program must demonstrate the ability to configure and deploy a node, and pass the minimum performance requirements as reported by the [Node Health Checker](/nodes/node-health-checker). 

### Summary steps

<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/install-node-and-register.svg'),
    dark: useBaseUrl('/img/docs/install-node-and-register.svg'),
  }}
/>

### Detailed steps

To participate in the AIT-3 program, follow the below steps. Use these steps as a checklist to keep track of your progress. Click on the links in each step for a detailed documentation.

1. Navigate to the [Aptos Community page](https://aptoslabs.com/community) to register or sign in to your account.

2. Install Petra (Aptos Wallet) follow the instruction [here](/guides/building-wallet-extension).
    
3. Create a wallet address using Petra, connect the wallet to your Aptos community account.

4. Complete the survey on Aptos community platform.

5. Read the Node Requirements. 

  Before you proceed, make sure that your hardware, storage and network resources satisfy the [Node Requirements](node-requirements.md).

6. Follow the instructions and deploy a validator node in the test mode.

  Follow the detailed node installation steps provided in: [Validators](/nodes/validator-node/validators). **Make sure to set your node in the Test mode.** Instructions are provided in the node installation sections. Test mode is required for Aptos Labs to do a health check on your node.

7. Register your node in the Aptos Community Platform.
   
  Navigate to the [Aptos Community page](https://aptoslabs.com/community) and register your node. Provide your account address, your operator public key, and your validator's network addresses. The operator public key is the  `account_public_key` from the "aptosbot.yaml" file for the validator node.

8. If your node passes healthcheck, you will be prompted to complete the identity verification process.

  The Aptos team will perform a node health check on your validator, using the [Node Health Checker](/nodes/node-health-checker). When Aptos confirms that your node is healthy, you will be asked to complete the KYC process. You will also be automatically enrolled in the Aptos notifications. This will enable you to receive all communication from Aptos Labs throughout the AIT-3 program.

9. Wait for the selection announcement. If you are selected, then proceed to **Initilize staking pool** step.


## Initialize staking pool

### Summary steps

<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/wallet-actions.svg'),
    dark: useBaseUrl('/img/docs/wallet-actions.svg'),
  }}
/>

### Detailed steps

:::caution Before you proceed
Proceed to the below steps only if you are selected to participate in the AIT-3.
:::

1. Confirm that you received the token from the Aptos team by checking the balance of your Petra wallet. Make sure you are connected to the AIT3 network by click **Settings** -> **Network**.

2. From the Chrome browser, go to the [**Proposals section** of the Aptos Governance page for AIT-3](https://explorer.devnet.aptos.dev/proposals?network=ait3). 

3. Install Petra (Aptos Wallet).
    
  Click on the **INSTALL WALLET** button and follow the directions to install the Aptos Wallet Extension on your Chrome browser. 
    
4. Create another wallet address for voter. This step is optional, you can use owner account as voter as well, however the best practice is to have a dedicate voting account so that you don't need to access your owner key frequently for governance operations.

5. **Next you will stake and delegate.** 

  :::tip Read the Staking document

  Make sure you read the [Staking](/concepts/staking) documentation before proceeding further. 
  :::

  You will begin by initializing the staking pool and delegating to the operator and the voter. 

    1. From the Chrome browser, go to the [**Staking section** of the Aptos Governance page for AIT-3](https://explorer.devnet.aptos.dev/proposals/stake?network=ait3).
    2. Make sure the wallet is connected with your **owner** account.
    3. Provide the following inputs:
        1. Staking Amount: 100000000000000 (1 million Aptos coin with 8 decimals)
        2. Operator Address: The address of your operator account. This is the `operator_account_address` from the "operator.yaml" file, under `~/$WORKSPACE/$USERNAME` folder.
        3. Voter Address: The wallet address of your voter.
    4. Click **SUBMIT**. You will see a green snackbar indicating that the transaction is successful.

6. Transfer 5000 coin to your operator account and voter account to pay gas fees using Petra wallet.

7. Next, proceed to **Connect to AIT-3 and join the validator set**.


## Connect to AIT-3 and join the validator set

### Detailed steps

See [Connecting to Aptos Incentivized Testnet](/nodes/ait/connect-to-testnet) for detailed steps.


## Vote

You will test the voting feature in this step.

1. From the Chrome browser, go to the [**Proposals section** of the Aptos Governance page for AIT-3](https://explorer.devnet.aptos.dev/proposals?network=test2).
2. View the proposals. When you are ready to vote on a proposal, click on the proposal. 
3. Make sure you connected the wallet with your **voter** wallet account. 
4. Provide your **owner** account address and vote “For” or “Against”. 
5. You will see a green snackbar indicating that the transaction is successful.


