---
title: "Use Remix IDE Plugin"
slug: "remix-ide-plugin"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Use Remix IDE Plugin

This tutorial details how to deploy and run Move modules on Remix IDE. It is a no-setup tool with a GUI for developing Move modules. The steps in summary are:

1. Connect to Remix IDE.
2. Select a chain.
3. Install a browser extension wallet.
4. Create the project. 
5. Compile and publish a Move module to the Aptos blockchain.
6. Interact with a Move module.
---

## Step 1: Connect to Remix IDE

[WELL DONE Code](https://docs.welldonestudio.io/code) is the official Remix IDE Plug-in. Please visit the [Remix IDE](https://remix.ethereum.org/) and follow the guide below.

Click **Plugin Manager** button in the left bar and search for **CODE BY WELL DONE STUDIO** and click the Activate button.

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

If you click the `Documentation` button, go to WELL DONE Docs, and if you find a problem or have any questions while using it, click the `Make an issue` button to go to the [Github Repository](https://github.com/welldonestudio/welldonestudio.github.io) and feel free to create an issue.

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

## Step 3: Install a browser extension wallet

:::info 
Petra wallet will be supported soon, and WELL DONE Wallet can be used now.
:::

After choosing a chain, click the `Connect to WELL DONE` button to connect to the **WELL DONE Wallet.** 

If you haven't installed the WELL DONE Wallet yet, please follow the following [manual](https://docs.welldonestudio.io/wallet/manual/) to install and create a wallet and create an account for the selected chain. Finally, go into the Setting tab of your wallet and activate Developer Mode.

And you must click the Refresh button in the upper right corner of the plug-in to apply changes to your wallet.

## Step 4: Create the Project

In Aptos, you can write smart contracts with Move language. **WELL DONE Code** provides two features to help developers new to Aptos and Move.

### Create Template

Create a simple example contract code written in Move. You can create a sample contract by selecting the template option and clicking the `Create a Template` button.

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

### New Project

Automatically generate a Move module structure. Write a name for the project, and click the `New Project` button to create a Move module structure.

:::info
You can create your own Move projects without using the features above. However, for the remix plugin to build and deploy the Move module, it must be built within the directory `aptos/`. If you start a new project, the structure should look like the following.
:::

  ```
  aptos
  └── <YOUR_PROJECT_NAME>
      ├── Move.toml
      └── sources
          └── YOUR_CONTRACT_FILE.move
  ```

## Step 5: Compile and publish a Move module to the Aptos blockchain.

1. Select the project you want to compile in the **PROJECT TO COMPILE** section.
2. Click the `Compile` button. Don't forget to write your address in Move.toml.

```toml
[package]
name = "Examples"
version = "0.0.0"

[addresses]
hello_blockchain = "your address"

[dependencies]
AptosFramework = { git = "https://github.com/aptos-labs/aptos-core.git", subdir = "aptos-move/framework/aptos-framework/", rev = "aptos-node-v1.2.0" }
```

3. When the compilation is complete, a compiled binary file is returned.

:::note
You can check the returned compiled binary file in `aptos/<YOUR_PROJECT_NAME>/out` directory.

If you need to revise the contract and compile again, delete the `out` directory and click the compile button.
:::

4. If you have a compiled contract code, then `Deploy` button will be activated. 

## Step 6: Interact with a Move module.

:::info
There are two ways to import contracts.
1. Automatically import contracts deployed through the above process.
2. Import existing deployed contracts through `At Address` button.
:::

1. You can check the modules and resources owned by the current account, and you can read the resources through the Get Resource button.
2. You can select a function, enter parameters as needed, and click a button to run the function. For the entry function, not the view function, a signature from the WELL DONE Wallet is required because the transaction signature and request are required.

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
