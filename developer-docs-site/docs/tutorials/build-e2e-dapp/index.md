---
title: "Build E2E Dapp on Aptos"
slug: "e2e-dapp-index"
---

# Build an End-to-End Dapp on Aptos

A common way to learn a new framework or programming language is to build a simple todo list. In this tutorial, we will learn how to build an end-to-end todo list dapp, starting from the smart contract side through the front-end side and finally use of a wallet to interact with the two.

See the completed code in the [my_first_dapp](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/my_first_dapp).

## Chapters

After meeting the [prerequisites](#prerequisites) and [getting set up](#setup) as described below, you will follow this tutorial in this order:

1. [Create a smart contract](./1-create-smart-contract.md)
2. [Set up React app](./2-set-up-react-app.md)
3. [Add Wallet support](3-add-wallet-support.md)
4. [Fetch Data from Chain](4-fetch-data-from-chain.md)
5. [Submit data to chain](./5-submit-data-to-chain.md)
6. [Handle Tasks](./6-handle-tasks.md)

## Prerequisites

You must have:

* [Aptos CLI](../../tools/aptos-cli/install-cli/index.md) `@1.0.4` or later
* [Aptos TypeScript SDK](../../sdks/ts-sdk/index.md) `@1.7.1` or later
* [Aptos Wallet Adapter](../../integration/wallet-adapter-concept.md) `@1.0.2` or later
* [Create React App](https://create-react-app.dev/)
* [node and npm](https://nodejs.org/en/)

Although we will explain some React decisions, we are not going to deep dive into how React works; so we assume you have some previous experience with React.

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

  Now let's [create a smart contract](./1-create-smart-contract.md).
