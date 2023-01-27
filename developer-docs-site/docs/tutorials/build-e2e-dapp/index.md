---
title: "Build E2E Dapp on Aptos"
slug: "e2e-dapp-index"
---

# Build an End-to-End Dapp on Aptos

A common way to learn a new framework or programming language is to build a simple todo list. In this tutorial, we will learn how to build an end-to-end todo list dapp, starting from the smart contract side through the front-end side and finally use of a wallet to interact with the two.

:::tip
The completed code is in the [todolist-dapp-toturial](https://github.com/aptos-labs/todolist-dapp-toturial)
:::

## Prerequisites

You must 

1. Aptos CLI `@1.0.4`
2. Aptos TS SDK `@1.6.0`
3. Aptos Wallet Adapter `@0.2.2`
4. Create React app

### Setup

In this section, we will create a `my-first-dapp` directory to hold our project files, both client-side code (React based)and the Move code (our smart contract).

1. Open a terminal and navigate to the desired directory for the project (for example, the `Desktop` directory).
2. Create a new directory called `my-first-dapp`, for example:
  ```shell
  mkdir my-first-dapp
  ```
3. Navigate into that directory:
  ```shell
  cd my-first-dapp
  ```

Now let's [create a smart contract](./1-create-smart-contract.md).