---
title: "Steps in AIT-3"
slug: "steps-in-ait3"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Steps in AIT-3

See below the summary flowcharts and detailed steps you will execute while participating in AIT-3.

:::caution Chrome browser only
The new Petra (Aptos Wallet) is supported only on the Chrome browser. Hence, for all the below AIT-3 tasks, make sure that you use only the Chrome browser.
:::

## Sign-in, connect Wallet and complete survey

Participants in the AIT-3 program must demonstrate the ability to configure and deploy a node, and pass the minimum performance requirements as reported by the [Node Health Checker](/nodes/node-health-checker/index).

### Summary steps

<center>
<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/sign-in-to-survey.svg'),
    dark: useBaseUrl('/img/docs/sign-in-to-survey-dark.svg'),
  }}
/>
</center>

### Detailed steps


To participate in the AIT-3 program, follow the below steps. Use these steps as a checklist to keep track of your progress. Click on the links in each step for a detailed documentation.

1. Navigate to the [Aptos Community page](https://aptoslabs.com/community) and follow the steps, starting with registering or signing in to your Discord account.

2. Before you click on Step 2 **CONNECT WALLET**:
   1. Delete any previous versions of Aptos Wallet you have installed on Chrome
   2. Install the Petra (Aptos Wallet) extension using Step 3 instructions, and
   2. Create the first wallet using Step 4 instructions.
3. **Install** the Petra (Aptos Wallet) extension on your Chrome browser by [following the instructions here](/guides/install-petra-wallet-extension).

4. <span id="create-wallet">Create the first wallet using Petra (Aptos Wallet)</span>.

  **This first wallet will always be the owner wallet**.

   1. Open the Aptos Wallet extension from the Extensions section of the Chrome browser, or by clicking on the puzzle piece on top right of the browser and selecting Aptos Wallet.
   2. Click **Create a new wallet**.
   3. Make sure to store your seed phrase somewhere safe. This account will be used in the future.

5. Click on Step 2 **CONNECT WALLET** to register the owner wallet address to your Aptos Community account. The Aptos team will airdrop coins to this owner wallet address.

6. Click on the Step 3 **COMPLETE SURVEY** to complete the survey.

7. Next, proceed to install and deploy the validator node.

## Deploy the validator node and register the node

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

  Before you proceed, make sure that your hardware, storage and network resources satisfy the [Node Requirements](node-requirements.md).
  :::

2. Follow the detailed node installation steps provided in [Validators](/nodes/validator-node/validators) and deploy a validator node in the test mode.

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

### Summary steps

<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/initialize-staking-pool.svg'),
    dark: useBaseUrl('/img/docs/initialize-staking-pool-dark.svg'),
  }}
/>

### Detailed steps

:::caution Before you proceed
Proceed to the below steps only if you are selected to participate in the AIT-3.
:::

1. Confirm that you received the token from the Aptos team by checking the balance of your Petra wallet. Make sure you are connected to the AIT-3 network by click **Settings** → **Network**.

2. Create another wallet address for the voter. See [the above Step 4: Create the wallet using Petra](#create-wallet) to create a wallet on Petra. This step is optional. You can use the owner wallet account as voter wallet as well. However, the best practice is to have a dedicate voting account so that you do not need to access your owner key frequently for governance operations.

3. <span id="stake-delegate"><b>Next you will stake and delegate.</b></span>

  :::tip Read the Staking document

  Make sure you read the [Staking](/concepts/staking) documentation before proceeding further.
  :::

  You will begin by initializing the staking pool and delegating to the operator and the voter.

    1. From the Chrome browser, go to the [**Staking section** of the Aptos Governance page for AIT-3](https://explorer.aptoslabs.com/proposals/staking?network=ait3).
    2. Make sure the wallet is connected with your **owner** account.
    3. Provide the following inputs:
        1. Staking Amount: 100000000000000 (1 million Aptos coin with 8 decimals)
        2. Operator Address: The address of your operator account. This is the `operator_account_address` from the "operator.yaml" file, under `~/$WORKSPACE/$USERNAME` folder.
        3. Voter Address: The wallet address of your voter.
    4. Click **SUBMIT**. You will see a green snackbar indicating that the transaction is successful.

6. Next, as the owner, using Petra wallet, transfer 5000 coin each to your operator address and voter wallet address. Both the operator and the voter will use these funds to pay the gas fees while validating and voting.

7. Proceed to **Connect to AIT-3 and join the validator set**.


## Connect to AIT-3 and join the validator set

### Detailed steps

See [Connecting to Aptos Incentivized Testnet](/nodes/ait/connect-to-testnet) for detailed steps.


## Vote

You will test the voting feature in this step.

1. From the Chrome browser, go to the [**Proposals section** of the Aptos Governance page for AIT-3](https://explorer.aptoslabs.com/proposals?network=ait3).
2. View the proposals. When you are ready to vote on a proposal, click on the proposal.
3. Make sure you connected the wallet with your **voter** wallet account.
4. Provide your **owner** account address and vote “For” or “Against”.
5. You will see a green snackbar indicating that the transaction is successful.

:::caution Before you proceed
The next steps can only be taken AFTER you have [initialized the Staking Pool](#stake-delegate).
:::

## Reset operator account
1. From the Chrome browser, go to the [Staking page](https://explorer.aptoslabs.com/proposals/staking?network=ait3)
2. Make sure the wallet is connected with your **owner** account.
3. Provide the **new operator** address in the input that says **New Operator Address**
4. click the **CHANGE OPERATOR** button. You will see a green snackbar indicating that the transaction is successful.

## Reset voter account
1. From the Chrome browser, go to the [Staking page](https://explorer.aptoslabs.com/proposals/staking?network=ait3)
2. Make sure the wallet is connected with your **owner** account.
3. Provide the **new voter** address in the input that says **New Voter Address**
4. click the **CHANGE VOTER** button. You will see a green snackbar indicating that the transaction is successful.

## Increase lockup duration
1. From the Chrome browser, go to the [Staking page](https://explorer.aptoslabs.com/proposals/staking?network=ait3)
2. Make sure the wallet is connected with your **owner** account.
3. click the **INCREASE LOCKUP** button. You will see a green snackbar indicating that the transaction is successful.
