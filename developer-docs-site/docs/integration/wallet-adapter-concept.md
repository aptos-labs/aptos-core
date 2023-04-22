---
title: "Integrate with Aptos Wallets"
id: "wallet-adapter-concept"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Integrate with Aptos wallets

Decentralized applications often run through a browser extension or mobile application to read onchain data and submit
transactions.  The Aptos Wallet Adapter allows for a single interface for apps and wallets to integrate together.

## Implementing the Aptos Wallet Adapter

For the best user experience, we suggest that dapps offer multiple wallets, to allow users to choose their preferred
wallet.

Implementing wallet integration can be difficult for dapps in:

1. Support and test all edge cases
2. Implement and maintain different wallet APIs
3. Provide users with needed functionality the wallet itself doesnt support
4. Keep track on all the different wallets in our ecosystem

In addition, creating and implementing a wallet is also not an easy task,

1. Provide a wallet that follows a known standard so it is easy to integrate with
2. Getting visibility and exposure in the ecosystem among all the other wallets
3. Dapp projects need to dedicate time and resource to integrate the wallet within their app

When we started building a wallet adapter, we wanted to provide an adapter that can be easy enough for wallets to integrate with and for dapps to use and implement.

For that, we provide an [Aptos Wallet Adapter](https://github.com/aptos-labs/aptos-wallet-adapter) monorepo for wallet and dapps creators to ease development and ensure a smooth process in building projects on the Aptos network.
The Aptos Wallet Adapter acts as a service between dapps and wallets and exposes APIs for dapps to interact with the wallets by following our [Wallet Standard](../standards/wallets.md). This in turns allows dapps to support many wallets with minimal integration efforts, and for wallets to follow a known standard and gain visibility.

## Adapter structure

The adapter has three different components, the:

1. Adapter Core package
2. Adapter React provider (for dapps)
3. Adapter Template plugin (for wallets)

This structure offers the following benefits:

- Modularity (separation of concerns) - separating the adapter into three components can help having more freedom in design, implementation, deployment and usage.
- Wallets create and own their plugin implementation (instead of having all in the same monorepo):
  - Reduces the packages bundle size used by dapps.
  - Lets them be self-service and support themselves without too much friction.
  - Prevents build failures in case of any bugs/bad implementation/wrong config files/etc.
- Simplicity - keeps the Provider package very light and small as the major logic is implemented in the core package.
- Flexibility - for wallets in creating and implementing custom functions.

### Adapter Core package

The [Adapter Core package](https://github.com/aptos-labs/aptos-wallet-adapter/tree/main/packages/wallet-adapter-core) handles the interaction between the dapp and the wallet. It:

- Exposes the standard API (and some different functions supported by different wallets)
- Holds the current wallet state and the installed wallets
- Emits events on different actions and much more

Dapps should not _know_ this package as dapps interact with the provider, which in turn interacts with the core package; some Types are exposed from the core package for the dapp to use.

Wallets should implement their own plugin class that extends the basic plugin class (properties + events) interface that lives in the core package.

:::tip
If a wallet supports functions that are not part of the basic plugin interface, a pull request should be made to the core package to include this function so it can support it. You can take a look at the `signTransaction` on the wallet core package for guidance.
:::

### Adapter React provider

The light [Adapter React package](https://github.com/aptos-labs/aptos-wallet-adapter/tree/main/packages/wallet-adapter-react) is for dapps to import and use. The package contains a `Provider` and a `Context` to implement and use within your app.

Follow the [Wallet Adapter For Dapp Builders](./wallet-adapter-for-dapp.md) guide on how to use the provider package on your dapp.

### Adapter Template plugin

Wallets looking to integrate with the adapter should implement their own wallet plugin, to ease the process we provide you with a pre-made class that implements the basic functionality needed (according to the wallet standard).

The [Wallet Adapter Plugin Template repo](https://github.com/aptos-labs/wallet-adapter-plugin-template) holds a pre-made class, a test file, and some config files to help you build and publish the plugin as an NPM package.

Follow the [Wallet Adapter For Wallet Builders](./wallet-adapter-for-wallets.md) on how to use the template to implement and publish your wallet plugin.

<center>
<ThemedImage
alt="Wallet Adapter Concept"
sources={{
    light: useBaseUrl('/img/docs/wallet-adapter-chart-light.svg'),
    dark: useBaseUrl('/img/docs/wallet-adapter-chart-dark.svg'),
  }}
/>
</center>
