---
title: "Create and Fund Accounts"
slug: "get-test-funds"
---

# Create and Fund Aptos Accounts

For testing purposes, you will want to create an Aptos account and fund it with testnet tokens. Much of this can be accomplished in the [wallet](https://github.com/aptos-foundation/ecosystem-projects#wallets) of your choice. We use the [Petra Wallet](https://petra.app/docs/use) here in combination with the [Aptos CLI](../cli-tools/aptos-cli-tool/index.md) to show you how they work together.

This document accompanies the command line instructions for the Aptos CLI on [creating](../cli-tools/aptos-cli-tool/use-aptos-cli.md#initialize-local-configuration-and-create-an-account) and [funding](../cli-tools/aptos-cli-tool/use-aptos-cli.md#fund-an-account-with-the-faucet) accounts with the [Aptos Faucet](https://github.com/aptos-labs/aptos-core/tree/main/crates/aptos-faucet), focusing upon the **testnet** network and leveraging a graphical wallet rather than the CLI.

To see how to interact with the Aptos Faucet in software, conduct [Your First Transaction](../tutorials/first-transaction.md) in **devnet**. To utilize your account in **mainnet**, you will first need to obtain the Aptos APT tokens.

Other than funding your account in mainnet, the rest of your workflow remains the same as in devnet and testnet. Just remember, in mainnet you are working with real, monetary value that cannot simply be recreated.

## Prerequisites

You will need these installed to proceed:

* The wallet of your choice; we use the [Petra Wallet](./install-petra-wallet.md) Chrome extension.
* [Aptos CLI](../cli-tools/aptos-cli-tool/index.md)

## Create an Aptos account

First, understand you can use your private key straight from the wallet and not have to import it from the CLI.

Note that you may create specific account types by passing the `--profile` argument and a unique name to `aptos-init` for special assignments later. Here we will create a `default` (typical) account.

1. Create the account on Aptos testnet to receive the NFT by running the following command and selecting `testnet`:
  ```shell
  aptos init
  ```
2. Receive the output:
  ```shell
  Configuring for profile default
  ```
3. When prompted for a network:
  ```shell
  Choose network from [devnet, testnet, mainnet, local, custom | defaults to devnet]
  ```
  Select `testnet` by entering it and hitting return. You may instead select `devnet` and get funds through the Aptos CLI later.
4. When prompted for your private key:
  ```shell
  Enter your private key as a hex literal (0x...) [Current: None | No input: Generate new key (or keep one if present)]
  ```
  Hit enter to generate a new key.
5. Receive output indicating success and resembling:
  ```shell
  No key given, generating key...
  Account a233bf7be2b93f1e532f8ea88c49e0c70a873d082890b6d9685f89b5e40d50c2 does not exist, you will need to create and fund the account through a community faucet e.g. https://aptoslabs.com/testnet-faucet, or by transferring funds from another account
  
  ---
  Aptos CLI is now set up for account a233bf7be2b93f1e532f8ea88c49e0c70a873d082890b6d9685f89b5e40d50c2 as profile default!  Run `aptos --help` for more information about commands
  {
    "Result": "Success"
  }
  ```
6. Note your configuration information can be found in `.aptos/config.yaml` relative to where you ran `aptos init`. Read that file to see each profile's private and public keys, account address, and REST API URL.

## Import the account into the CLI

Here we will add the account to your [wallet](https://github.com/aptos-foundation/ecosystem-projects#wallets). We use the [Petra Wallet](./install-petra-wallet.md) Chrome extension here:

1. Read `.aptos/config.yaml` to see and copy the `default` private key.
3. Open the wallet and select the [Testnet network](https://petra.app/docs/use) in the wallet via *Petra settings > Network > Testnet*.
4. Go to *Petra > Settings > Switch account > Add Account > Import private key*.
5. Paste the `default` private key there.
6. Click **Submit** to add the previously created account to the wallet.
7. You are switched into that account automatically.

## Get coins from the faucet

1. Go to the *Petra > Settings > Network > Testnet* network if not there already.
2. Connect your wallet to the Aptos faucet at https://aptoslabs.com/testnet-faucet:
  ![Faucet connect](/img/connect-wallet-faucet.png "Connect faucet to wallet")
3. Select your wallet type:
  ![Wallet select](/img/select-wallet-faucet.png "Select your wallet for faucet")
4. Approve the connection request:
  ![Faucet approval](/img/approve-wallet-faucet.png "Approve connecting faucet to wallet")
5. Now when you load your wallet, you will see a **Faucet** button next to **Send**. Click **Faucet** to receive one APT per click to use when minting.
