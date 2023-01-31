---
title: "Build E2E Dapp on Aptos"
slug: "e2e-dapp-index"
---

# Build an End-to-End Dapp on Aptos

A common way to learn a new framework or programming language is to build a simple todo list. In this tutorial, we will learn how to build an end-to-end todo list dapp, starting from the smart contract side through the front-end side and finally use of a wallet to interact with the two.

## Prerequisites

You must have:

1. [Aptos CLI](../../cli-tools/aptos-cli-tool/index.md) `@1.0.4`
2. [Aptos TypeScript SDK](../../sdks/ts-sdk/index.md) `@1.6.0`
3. [Aptos Wallet Adapter](../../concepts/wallet-adapter-concept.md) `@0.2.2`
4. [Create React App](https://create-react-app.dev/)

See also the completed code is in the [todolist-dapp-tutorial](https://github.com/aptos-labs/todolist-dapp-tutorial).

## Setup

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

## Chapters

Follow this tutorial in this order:

1. [Create a smart contract](./1-create-smart-contract.md)
2. [Set up React app](./2-set-up-react-app.md)
3. [Add Wallet support](3-add-wallet-support.md)
4. [Fetch Data from Chain](4-fetch-data-from-chain.md)
5. [Submit data to chain](./5-submit-data-to-chain.md)
6. [Handle Tasks](./6-handle-tasks.md)

Now let's [create a smart contract](./1-create-smart-contract.md).