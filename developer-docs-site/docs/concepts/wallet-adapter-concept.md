---
title: "Wallet Adapter Concept"
id: "wallet-adapter-concept"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Wallet Adapter Concept

Aptos provides a [monorepo](https://github.com/aptos-labs/aptos-wallet-adapter) “Aptos Wallet Adapter” for wallet and dapps creators for easy development and smooth process in building projects on the Aptos network.

The Aptos Wallet Adapter acts as a service between dapps and wallets and exposes APIs for dapps to interact with the wallets by following our [Wallet Standard](../guides/wallet-standard).

## Adapter Structure

The adapter has 3 different components

1. The Adapter Core Package
2. The Adapter React Provider (for dapps)
3. The Adapter Template Plugin (for wallets)

### Adapter Core Package

This is the [core package](https://github.com/aptos-labs/aptos-wallet-adapter/tree/main/packages/wallet-adapter-core) that handles the interaction between the dapp and the wallet.

- Exposes the standard API (and some different functions supported by different wallets)
- Holds the current wallet state and the installed wallets,
- Emits events on different actions and much more!

Dapps should not “know” this package as dapps interact with the provider which interacts with the core package, some Types are exposed from the core package for the dapp to use.

Wallets should implement their own plugin class that extends the basic plugin class (props + events) interface that lives in the core package.

:::tip
If a wallet supports functions that are not part of the basic plugin interface, a PR should be made to the core package to include this function so it can support it. You can take a look at the `signTransaction` on the wallet core package
:::

### Adapter React Provider

This is a light [React package](https://github.com/aptos-labs/aptos-wallet-adapter/tree/main/packages/wallet-adapter-react) for dapps to import and use. The package contains a `Provider` and a `Context` to implement and use within your app.

Follow the [guide](../guides/wallet-adapter-for-dapp.md) on how to use the provider package on your dapp!

### Adapter Template Plugin

Wallets looking to integrate with the adapter should implement their own wallet plugin, to ease the process we provide you with a pre-made class that implements the basic functionality needed (according to the wallet standard).

The [Wallet Adapter Plugin Template repo](https://github.com/aptos-labs/wallet-adapter-plugin-template) holds a pre-made class, a test file, and some config files to help you build and publish the plugin as a npm package.

Follow the [guide](../guides/wallet-adapter-for-wallets.md) on how to use the template to implement and publish your wallet plugin!

### Why we use this structure?

When we started to think about building a wallet adapter, we wanted a structure that can be easy enough for wallets to integrate with and for dapps to use and implement.

We did research on the existing adapters, got some feedbacks from our community and came up with this structure which we believe can help wallets and dapps in development and when going to production.

1. Modularity (Separation of concerns) - separate the adapter into 3 components can help having more freedom in design, implementation, deployment and usage.
2. Wallets create and own their plugin implementation (instead of having all in the same monorepo)
   - Reduces the packages bundle size used by dapps
   - Let them be self service / support themselves without too much friction
   - Prevents build failures in case of any bugs/bad implementation/wrong config files/etc
3. Keeps the Provider package very light and small as the major logic implemented in the core package.
4. Flexibility for wallets in creating and implementing custom functions

<center>
<ThemedImage
alt="Wallet Adapter Concept"
sources={{
    light: useBaseUrl('/img/docs/10-adapter-chart-light.svg'),
    dark: useBaseUrl('/img/docs/10-adapter-chart-dark.svg'),
  }}
/>
</center>
