---
title: "For Dapps"
id: "wallet-adapter-for-dapp"
---

# Wallet Adapter For Dapp builders

Imagine you have a great idea for a dapp and you want to start building it, eventually you would probably need to integrate wallet or multiple wallets so your users can interact with the Aptos chain.
Implementing wallet integration can be difficult in supporting all edge cases, new features, unsupported functionality and it can be even harder to support multiple wallets!

In addition, different wallets have different APIs and not all wallets share the same naming convention. For example, maybe all wallets have a `connect` method, but not all wallets call that method `connect`, that can be tricky to support!

Luckily, Aptos built a wallet adapter, created and maintained by the Aptos team, to help you with that!

The adapter provides

- Easy wallet implementation - no need to implement and support code for multiple wallets
- Support for different wallet APIs
- Features support that are not implemented on the wallet level
- Provides detection for uninstalled wallets (so you can show users that a wallet is not installed)
- Provides autoConnect functionality and remembers the current wallet state.
- Listens to wallet events such as Account change and Network change
- Developed and maintained by the Aptos ecosystem team

### Usage

Currently the adapter supports a `React provider` for you to include in your app.

Install wallet dependencies you want to include in your app. You can find a list of the wallets [here](https://github.com/aptos-labs/aptos-wallet-adapter#supported-wallet-packages)

Install the react provider

```bash
npm install @aptos-labs/wallet-adapter-react
```

Import dependencies.
On the App.jsx file, Import the installed wallets.

```js
import { AptosWallet } from "some-aptos-wallet-package";
```

Import the AptosWalletAdapterProvider.

```js
import { AptosWalletAdapterProvider } from "@aptos-labs/wallet-adapter-react";
```

Wrap your app with the Provider, pass it the plugins (wallets) you want to have on your app as an array and a autoConnect option (set to false by default)

```js
const wallets = [new AptosWallet()];
<AptosWalletAdapterProvider plugins={wallets} autoConnect={true}>
  <App />
</AptosWalletAdapterProvider>;
```

Use wallet
On any page you want to use the wallet props, import useWallet from @aptos-labs/wallet-adapter-react

```js
import { useWallet } from "@aptos-labs/wallet-adapter-react";
```

Then you can use the exported properties

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

Then, You can use the [examples](https://github.com/aptos-labs/aptos-wallet-adapter/tree/main/packages/wallet-adapter-react#examples) on the package README file
