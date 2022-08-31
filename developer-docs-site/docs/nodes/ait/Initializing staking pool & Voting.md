---
title: "Connecting to Aptos Incentivized Testnet"
slug: "initializing-staking-pool-and-voting"
sidebar_position: 14
---

## Initializing staking pool & Voting

In AIT3 we will have UI support to allow owner managing the staking pool, see details [here](https://aptos.dev/nodes/ait/steps-in-ait3#initialize-staking-pool-and-voting). We have also created a voting UI, which you can see from below

Once you have completed the following sections, you can go to [Connecting to Aptos Incentivized Testnet](/nodes/ait/connect-to-testnet) for detailed steps on how to get your node connected to the blockchain

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

    1. From the Chrome browser, go to the [**Staking section** of the Aptos Governance page for AIT-3](https://explorer.devnet.aptos.dev/proposals/staking?network=ait3).
    2. Make sure the wallet is connected with your **owner** account.
    3. Provide the following inputs:
        1. Staking Amount: 100000000000000 (1 million Aptos coin with 8 decimals)
        2. Operator Address: The address of your operator account. This is the `operator_account_address` from the "operator.yaml" file, under `~/$WORKSPACE/$USERNAME` folder.
        3. Voter Address: The wallet address of your voter.
    4. Click **SUBMIT**. You will see a green snackbar indicating that the transaction is successful.

6. Next, as the owner, using Petra wallet, transfer 5000 coin each to your operator address and voter wallet address. Both the operator and the voter will use these funds to pay the gas fees while validating and voting.

7. Proceed to **Connect to AIT-3 and join the validator set**.


## Staking with CLI

We now have a UI to support some staking operation, but in any case if you need to do operations not supported in UI, you can use CLI for it.

- Initialize CLI with your wallet private key or create new wallet

  ```
  aptos init --profile ait3-owner \
    --rest-url http://ait3.aptosdev.com
  ```

  You can either enter the private key from an existing wallet, or create new wallet address depends on your need.

- Initialize staking pool using CLI

  ```
  aptos stake initialize-stake-owner \
    --initial-stake-amount 100000000000000 \
    --operator-address <operator-address> \
    --voter-address <voter-address> \
    --profile ait3-owner
  ```

- Transfer coin between accounts. Don't forget to transfer some coin to your operator account to pay gas.

  ```
  aptos account create --account <operator-account> --profile ait3-owner
  
  aptos account transfer \
  --account <operator-account> \
  --amount 5000 \
  --profile ait3-owner
  ```
  
## Vote

You will test the voting feature in this step.

1. From the Chrome browser, go to the [**Proposals section** of the Aptos Governance page for AIT-3](https://explorer.devnet.aptos.dev/proposals?network=ait3).
2. View the proposals. When you are ready to vote on a proposal, click on the proposal. 
3. Make sure you connected the wallet with your **voter** wallet account. 
4. Provide your **owner** account address and vote “For” or “Against”. 
5. You will see a green snackbar indicating that the transaction is successful.

:::caution Before you proceed
The next steps can only be taken AFTER you have [initialized the Staking Pool](#stake-delegate).
:::

## Reset operator account
1. From the Chrome browser, go to the [Staking page](https://explorer.devnet.aptos.dev/proposals/staking?network=ait3)
2. Make sure the wallet is connected with your **owner** account.
3. Provide the **new operator** address in the input that says **New Operator Address**
4. click the **CHANGE OPERATOR** button. You will see a green snackbar indicating that the transaction is successful.

## Reset voter account
1. From the Chrome browser, go to the [Staking page](https://explorer.devnet.aptos.dev/proposals/staking?network=ait3)
2. Make sure the wallet is connected with your **owner** account.
3. Provide the **new voter** address in the input that says **New Voter Address**
4. click the **CHANGE VOTER** button. You will see a green snackbar indicating that the transaction is successful.

## Increase lockup duration
1. From the Chrome browser, go to the [Staking page](https://explorer.devnet.aptos.dev/proposals/staking?network=ait3)
2. Make sure the wallet is connected with your **owner** account.
3. click the **INCREASE LOCKUP** button. You will see a green snackbar indicating that the transaction is successful.

# Some additional CLI commands

- Switch operator

  ```
  aptos stake set-operator \
    --operator-address <new-operator-address> \ 
    --profile ait3-owner
  ```

- Switch voter

  ```
  aptos stake set-delegated-voter \
    --voter-address <new-voter-address> \ 
    --profile ait3-owner
  ```

- Add stake

  ```
  aptos stake add-stake \
    --amount <amount> \
    --profile ait3-owner \
    --max gas 5000 # You can adjust the amount of max gas here.
  ```

- Increase stake lockup

  ```
  aptos stake increase-lockup --profile ait3-owner
  ```

- Unlock stake

  ```
  aptos stake unlock-stake \
    --amount <amount> \
    --profile ait3-owner
  ```

- Withdraw stake

  ```
  aptos stake withdraw-stake \
    --amount <amount> \
    --profile ait3-owner
  ```
