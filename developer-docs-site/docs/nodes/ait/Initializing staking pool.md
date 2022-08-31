---
title: "Connecting to Aptos Incentivized Testnet"
slug: "connect-to-testnet"
sidebar_position: 14
---

## Initializing staking pool

In AIT3 we will have UI support to allow owner managing the staking pool, see details [here](https://aptos.dev/nodes/ait/steps-in-ait3#initialize-staking-pool), if you've already done this through the UI, you can igore this step and jump into "Bootstrapping validator node". 

Alternatively, you can also use CLI to intialize staking pool:

- Initialize CLI with your wallet **private key**, you can get in from Settings -> Credentials

  ```
  aptos init --profile ait3-owner \
    --rest-url https://ait3.aptosdev.com
  ```

- Initialize staking pool using CLI

  ```
  aptos stake initialize-stake-owner \
    --initial-stake-amount 100000000000000 \
    --operator-address <operator-address> \
    --voter-address <voter-address> \
    --profile ait3-owner
  ```

- Don't forget to transfer some coin to your operator account to pay gas, you can do that with Petra, or CLI

  ```
  aptos account create --account <operator-account> --profile ait3-owner
  
  aptos account transfer \
  --account <operator-account> \
  --amount 5000 \
  --profile ait3-owner
  ```
  
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

- Transfer coin between accounts

  ```
  aptos account transfer \
    --account <operator-address> \
    --amount <amount> \
    --profile ait3-owner
  ```

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
    --max gas 5000 # you can play around with the max gas here
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

