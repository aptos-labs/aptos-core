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

## Deploy the validator node and register 

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

1. Read the Node Requirements. 

  Before you proceed, make sure that your hardware, storage and network resources satisfy the [Node Requirements](node-requirements.md).

2. Follow the instructions and deploy a validator node in the test mode.

  Follow the detailed node installation steps provided in: [Validators](/nodes/validator-node/validators). **Make sure to set your node in the Test mode.** Instructions are provided in the node installation sections. Test mode is required for Aptos Labs to do a health check on your node.

3. Register your node in the Aptos Community Platform.
   
  Navigate to the [Aptos Community page](https://aptoslabs.com/community) and register your node. Provide your account address, your operator public key, and your validator's network addresses. The operator public key is the  `account_public_key` from the "aptosbot.yaml" file for the validator node.

4. If your node passes healthcheck, you will be prompted to complete the KYC process.

  The Aptos team will perform a node health check on your validator, using the [Node Health Checker](/nodes/node-health-checker). When Aptos confirms that your node is healthy, you will be asked to complete the KYC process. You will also be automatically enrolled in the Aptos notifications. This will enable you to receive all communication from Aptos Labs throughout the AIT-3 program.

5. If you are selected, then proceed to **Install the Wallet and Stake** step.


## Install the Wallet and Stake

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
Proceed to the below steps only if your node has passed the NHC for AIT-3 and you are selected to participate in the AIT-3.
:::

1. From the Chrome browser, go to the [**Proposals section** of the Aptos Governance page for AIT-3](https://explorer.devnet.aptos.dev/proposals?network=test2). 

2. Install Petra (Aptos Wallet).
    
  Click on the **INSTALL WALLET** button and follow the directions to install the Aptos Wallet Extension on your Chrome browser. 
    
3. Create wallets for the owner and the voter. 

  In this step you will create wallets for two personas: the owner and the voter. See [Owner, operator and voter personas](/nodes/ait/whats-new-in-ait3#owner-operator-and-voter-personas) for an explanation of personas. 
    
    1. Create the first wallet. **This first wallet will always be the owner wallet**.
        1. Open the Aptos Wallet extension from the Extensions section of the Chrome browser, or by clicking on the puzzle piece on top right of the browser and selecting Aptos Wallet.
        2. Click **Create a new wallet**. 
        3. When you are done creating the wallet, go to **Extensions** > **Aptos Wallet** and click on the gear icon on the bottom right. You will see the **Settings** screen. Click on the **Network** button and select **Localhost** network. 
        4. Next, click again on the gear icon on the bottom right to go to the **Settings** page. Click on the **Credentials**. Copy and paste and save both the **Private key** and **Address** keys in a separate text file for later use. **This private key is the owner key**.
    2. Create the voter wallet. 
        1. Open the Aptos Wallet extension. 
        2. Click on the pair of 3 vertical dots at the top right of the Aptos Wallet screen. A slide-up screen will appear showing **+New Wallet** button. Create a new wallet and name it as **voter**.  
        3. Save the **Private key** and **Address** for the voter wallets. See the first wallet creation step above that describes how to save. 
        4. Now you have two wallets: For the owner and the voter.
4. Register the owner wallet address in the Aptos Community Platform. The Aptos team will airdrop coins to this owner wallet address. This step will also establish that you are the owner for the node you registered earlier (see Step 3 above in the **Deploy the validator node** section).
5. Wait until the Aptos team airdrops the coins into the owner wallet before proceeding further. Proceed to the next step, i.e., staking, after you verify that the airdropped coins are in the owner wallet. 
6. **Next you will stake and delegate.** 

  :::tip Read the Staking document

  Make sure you read the [Staking](/concepts/staking) documentation before proceeding further. 
  :::

  You will begin by initializing the staking pool and delegating to the operator and the voter. 

    1. From the Chrome browser, go to the [**Staking section** of the Aptos Governance page for AIT-3](https://explorer.devnet.aptos.dev/proposals/stake?network=test2).
    2. Make sure the wallet is connected with your **owner** account.
    3. Provide the following inputs:
        1. Staking Amount: TODO
        2. Operator Address: The address of your operator (i.e., validator). This is the `account_address` from the "aptosbot.yaml" file.
        3. Voter Address: The wallet address of your voter.
    4. Click **SUBMIT**. You will see a green snackbar indicating that the transaction is successful.
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


