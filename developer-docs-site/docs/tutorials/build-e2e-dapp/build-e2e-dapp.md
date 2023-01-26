---
title: "Build an e2e dapp on Aptos"
slug: "build-an-e2e-dapp-on-aptos"
---

# Build an e2e dapp on Aptos

A common way to learn a new framework or a new language is to build a simple todo list. In this tutorial we will learn how to build an e2e todo list dapp, starting from the smart contract side to the front-end side and use Wallet to interact between the two.

What we use:

1. Aptos `CLI@1.0.4`
2. Aptos `TS SDK@1.6.0`
3. Aptos Wallet `Adapter@0.2.2`
4. create react app

### Set up the project

1. Open a terminal and go to wherever you want the project to be in (for example, the `Desktop` folder).
2. Create a new folder called `my-first-dapp`(on Mac we can do it with `mkdir my-first-dapp`)
3. `cd my-first-dapp`

`my-first-dapp` folder will hold our project files - `client side code (react based)` and the `move` code (our smart contract).

:::tip
The completed code is in this [Github repo](https://github.com/aptos-labs/todolist-dapp-toturial)
:::
