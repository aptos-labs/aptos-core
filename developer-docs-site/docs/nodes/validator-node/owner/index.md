---
title: "Owner"
slug: "index"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Owner (Owner actions only - DRAFT)

:::caution Chrome browser only
The new Petra (Aptos Wallet) is supported only on the Chrome browser. 
:::

***This is the landing page for the owner persona. I will edit this page. For now, only a placeholder.***

## Sign-in and connect Wallet

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


1. Navigate to the [Aptos Community page](https://aptoslabs.com/community) and follow the steps, starting with registering or signing in to your Discord account.

2. Before you click on Step 2 **CONNECT WALLET**:
   1. Delete any previous versions of Aptos Wallet you have installed on Chrome
   2. Install the Petra (Aptos Wallet) extension using Step 3 instructions, and
   3. Create the first wallet using Step 4 instructions.
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
REMOVE FROM OWNER SECTION

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

7. DEFINE NEXT STEP.


## Connect to AIT-3 and join the validator set

REMOVE FROM THE OWNER SECTION.

## Owner actions

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

## Staking with CLI

:::tip Stake with UI
You can also use UI to perform a few staking operations. Proceed below to use the CLI to perform staking operations. 
:::

- Initialize CLI with your wallet private key or create new wallet

  ```bash
  aptos init --profile testnet-owner \
    --rest-url http://testnet.aptoslabs.com
  ```

  You can either enter the private key from an existing wallet, or create new wallet address depends on your need.

- Initialize staking pool using CLI

  ```bash
  aptos stake initialize-stake-owner \
    --initial-stake-amount 100000000000000 \
    --operator-address <operator-address> \
    --voter-address <voter-address> \
    --profile testnet-owner
  ```

- Transfer coin between accounts

  ```bash
  aptos account transfer \
    --account <operator-address> \
    --amount <amount> \
    --profile testnet-owner
  ```

- Switch operator

  ```bash
  aptos stake set-operator \
    --operator-address <new-operator-address> \ 
    --profile testnet-owner
  ```

- Switch voter

  ```bash
  aptos stake set-delegated-voter \
    --voter-address <new-voter-address> \ 
    --profile testnet-owner
  ```

- Add stake

  ```bash
  aptos stake add-stake \
    --amount <amount> \
    --profile testnet-owner \
    --max-gas 10000
  ```

  :::tip Max gas
    You can adjust the above `max-gas` number. Ensure that you sent your operator enough tokens to pay for the gas fee.
    :::

- Increase stake lockup

  ```bash
  aptos stake increase-lockup --profile testnet-owner
  ```

- Unlock stake

  ```bash
  aptos stake unlock-stake \
    --amount <amount> \
    --profile testnet-owner
  ```

- Withdraw stake

  ```bash
  aptos stake withdraw-stake \
    --amount <amount> \
    --profile testnet-owner
  ```
