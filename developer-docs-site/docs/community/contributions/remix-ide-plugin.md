---
title: "Use Remix IDE Plugin"
slug: "remix-ide-plugin"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Use Remix IDE Plugin

This tutorial explains how to deploy and run Move modules with the [WELLDONE Code Remix IDE](https://docs.welldonestudio.io/code) plugin. This tool offers a graphical interface for developing Move [modules](../../move/book/modules-and-scripts.md#modules). 

Here are the steps to use the Remix IDE plugin for Move (described in detail below):

1. [Connect to Remix IDE](#step-1-connect-to-remix-ide).
2. [Select a chain](#step-2-select-a-chain).
3. [Install a browser extension wallet](#step-3-install-a-wallet).
4. [Create the project](#step-4-create-the-project). 
5. [Compile and publish a Move module to the Aptos blockchain](#step-5-compile-and-publish-a-move-module-to-the-aptos-blockchain).
6. [Interact with a Move module](#step-6-interact-with-a-move-module).

## Step 1: Connect to Remix IDE

1. Load the [Remix IDE](https://remix.ethereum.org/).

2. Accept or decline the personal information agreement and dismiss any demonstrations.

3. Click the **Plugin Manager** button near the bottom left, search for *CODE BY WELLDONE STUDIO*, and click **Activate**.

<center>
<ThemedImage
alt="Remix IDE plugin"
sources={{
    light: useBaseUrl('/img/docs/remix-ide-plugin.png'),
    dark: useBaseUrl('/img/docs/remix-ide-plugin.png'),
  }}
width= "80%"
/>
</center>

## Step 2: Select a Chain

Click the newly created icon at the bottom of the left menu. Then, select **Aptos (MoveVM)** from the chain list.

<center>
<ThemedImage
alt="Remix Select a chain"
sources={{
    light: useBaseUrl('/img/docs/remix-select-chain.png'),
    dark: useBaseUrl('/img/docs/remix-select-chain.png'),
  }}
width="50%"
/>
</center>

## Step 3: Install a wallet

WELLDONE Wallet can be used with the Remix IDE plugin now, with support for [Petra wallet](https://petra.app/) coming soon. See the list of [Aptos wallets](https://github.com/aptos-foundation/ecosystem-projects#wallets) available in the ecosystem.

This steps assumes you are using the WELLDONE Wallet. Follow [the manual](https://docs.welldonestudio.io/wallet/manual/) to install the wallet and create an account for the Aptos blockchain. Once that is done, follow these steps:

1. Choose a network (e.g. devnet) in the dropdown menu at the top of the main tab.
1. Go into the **Settings** tab of your wallet and activate **Developer Mode**.

Now in the Remix UI click the **Connect to WELLDONE** button to connect to the **WELLDONE Wallet**. 

Click the **Refresh** button in the upper right corner of the plug-in to apply changes to your wallet.

## Step 4: Create the Project

In Aptos, you can write smart contracts with the [Move programming language](../../move/move-on-aptos.md). **WELLDONE Code** provides two features to help developers new to Aptos and Move.

### Select a template

Create simple example contract code written in Move. You can create a sample contract by selecting the *template* option and clicking the **Create** button.

<center>
<ThemedImage
alt="Remix Template Code"
sources={{
    light: useBaseUrl('/img/docs/remix-template-code.png'),
    dark: useBaseUrl('/img/docs/remix-template-code.png'),
  }}
width="50%"
/>
</center>

### Create a new project

Automatically generate the Move module structure. Write a name for the project, and click the **Create** button to create a Move module structure.

:::info
You can create your own Move projects without using the features above. However, for the Remix IDE plugin to build and deploy the Move module, it must be built within the directory `aptos/`. If you start a new project, the structure should resemble:
:::

  ```
  aptos
  └── <YOUR_PROJECT_NAME>
      ├── Move.toml
      └── sources
          └── YOUR_CONTRACT_FILE.move
  ```

## Step 5: Compile and publish a Move module to the Aptos blockchain

1. Select the project you want to compile in the **PROJECT TO COMPILE** section.
2. Add your address to the `Move.toml` file.
3. Click the `Compile` button. 

```toml
[package]
name = "Examples"
version = "0.0.0"

[addresses]
hello_blockchain = "your address"

[dependencies]
AptosFramework = { git = "https://github.com/aptos-labs/aptos-core.git", subdir = "aptos-move/framework/aptos-framework/", rev = "aptos-node-v1.2.0" }
```

4. When the compilation is complete, a compiled binary file is returned in the `aptos/<YOUR_PROJECT_NAME>/out` directory.

If you need to revise the contract and compile again, delete the `out` directory and click **Compile** once more.

5. Once you have compiled contract code, the `Deploy` button will be activated.

## Step 6: Interact with a Move module

:::info
There are two ways to import contracts.
1. Automatically import contracts deployed through the above process.
2. Import existing deployed contracts through the **At Address** button.
:::

1. Check the modules and resources owned by the current account and read the resources through the **Get Resource** button.
2. You can select a function, enter parameters as needed, and click a button to run the function. For an entry function - not the view function - a signature from the WELLDONE Wallet is required because the transaction signature and request are required.

<center>
<ThemedImage
alt="Remix View Function"
sources={{
    light: useBaseUrl('/img/docs/remix-view-function.png'),
    dark: useBaseUrl('/img/docs/remix-view-function.png'),
  }}
/>

<ThemedImage
alt="Remix Entry Function"
sources={{
    light: useBaseUrl('/img/docs/remix-entry-function.png'),
    dark: useBaseUrl('/img/docs/remix-entry-function.png'),
  }}
/>
</center>

## Get support

Click the **Documentation** button to seek help with this Remix IDE plugin. To file requests, click the **Make an issue** button to go to the [welldonestudio](https://github.com/welldonestudio/welldonestudio.github.io) GitHub Repository and [file an issue](https://github.com/welldonestudio/welldonestudio.github.io/issues/new/choose).
