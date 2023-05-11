---
title: "For Dapps"
id: "wallet-adapter-for-dapp"
---

# Wallet Adapter For Dapp Builders

Imagine you have a great idea for a dapp and you want to start building it. Eventually, you will need to integrate a wallet or multiple wallets so your users can interact with the Aptos blockchain.
Implementing wallet integration can be difficult in supporting all edge cases, new features, unsupported functionality. And it can be even harder to support multiple wallets.

In addition, different wallets have different APIs, and not all wallets share the same naming convention. For example, maybe all wallets have a `connect` method, but not all wallets call that method `connect`; that can be tricky to support.

Luckily, Aptos built a wallet adapter, created and maintained by the Aptos team, to help you ramp up development and standardize where possible.

The Aptos Wallet Adapter provides:

- Easy wallet implementation - no need to implement and support code for multiple wallets.
- Support for different wallet APIs.
- Support for features not implemented on the wallet level.
- Detection for uninstalled wallets (so you can show users that a wallet is not installed).
- Auto-connect functionality and remembers the current wallet state.
- Listens to wallet events, such as account and network changes.
- A well-developed and maintained reference implementation by the Aptos ecosystem team.

## Install

Currently, the adapter supports a _React provider_ for you to include in your app.

Install wallet dependencies you want to include in your app. You can find a list of the wallets in the Aptos Wallet Adapter [README](https://github.com/aptos-labs/aptos-wallet-adapter#supported-wallet-packages).

Install the React provider:

```bash
npm install @aptos-labs/wallet-adapter-react
```

## Import dependencies

In the `App.jsx` file:

Import the installed wallets:

```js
import { PetraWallet } from "petra-plugin-wallet-adapter";
```

Import the `AptosWalletAdapterProvider`:

```js
import { AptosWalletAdapterProvider } from "@aptos-labs/wallet-adapter-react";
```

Wrap your app with the Provider, pass it the plugins (wallets) you want to have on your app as an array, and include an autoConnect option (set to false by default):

```js
const wallets = [new PetraWallet()];
<AptosWalletAdapterProvider plugins={wallets} autoConnect={true}>
  <App />
</AptosWalletAdapterProvider>;
```

### Use

On any page you want to use the wallet properties, import `useWallet` from `@aptos-labs/wallet-adapter-react`:

```js
import { useWallet } from "@aptos-labs/wallet-adapter-react";
```

You can then use the exported properties:

```js
const {
  connect,
  account,
  network,
  connected,
  disconnect,
  wallet,
  wallets,
  signAndSubmitTransaction,
  signTransaction,
  signMessage,
} = useWallet();
```

Finally, use the [examples](https://github.com/aptos-labs/aptos-wallet-adapter/tree/main/packages/wallet-adapter-react#examples) on the package README file to build more functionality into your dapps.
