---
title: "Add Wallet suport"
id: "add-wallet-support"
---

# Add Wallet support

Wallet is a program that used to submit a transaction to chain.

Aptos provides a [wallet adapter](https://github.com/aptos-labs/aptos-wallet-adapter) that saves us time and work in implementing wallets logic and a UI package we can use to add a wallet connect button and a wallet selector modal.

You can read about it more in the [Wallet Adapter Concept](../../concepts/wallet-adapter-concept) page.

1. Stop the local server if running
2. In the `client` folder run

```cmd
npm i @aptos-labs/wallet-adapter-react@0.2.2
```

```cmd
npm i @aptos-labs/wallet-adapter-ant-design@0.1.0
```

That installs 2 packages for us

- the adapter react provider that holds the logic
- a wallet connect UI package

3. We now need to add wallets to our app. There is a [list](https://github.com/aptos-labs/aptos-wallet-adapter#supported-wallet-packages) of wallets the adapter supports, but to keep this tutorial simple, we would use only one wallet.
   Still in the `client` folder, run

```cmd
npm i petra-plugin-wallet-adapter
```

:::tip
If you haven't installed the Petra wallet extension yet:

1. See the [user instructions](https://petra.app/docs/use) on petra.app for help.
2. Switch to the Devnet network by clicking, settings, network, and selecting **devnet**.
3. Click the faucet button to ensure you can receive test tokens.

:::

4. Open `Index.tsx` file. At the top of the file add the following

```js
import { PetraWallet } from "petra-plugin-wallet-adapter";
import { AptosWalletAdapterProvider } from "@aptos-labs/wallet-adapter-react";
```

5. Still on `Index.tsx`, add a const that holds an array of wallets

```js
...
const wallets = [new PetraWallet()];
...
```

6. Inside the `render` method, update the code with the following

```js
...
<AptosWalletAdapterProvider plugins={wallets} autoConnect={true}>
  <App />
</AptosWalletAdapterProvider>
...
```

That wraps our app with the adapter provider and initialize it with our wallets. It also sets the provider to autoConnect a wallet.

7. Open the App.tsx file, and import the wallet connect UI package we installed in the previous step. At the top of the file add the following

```js
import { WalletSelector } from "@aptos-labs/wallet-adapter-ant-design";
```

8. The UI package uses a style .css file, lets import that one also at the bottom of the import statements

```js
...
import "@aptos-labs/wallet-adapter-ant-design/dist/index.css";
```

9. In the `return` statement, remove the `<h1>Connect Wallet</h1>` text, and add the WalletSelector component

```js
...
<Col span={12} style={{ textAlign: "right", paddingRight: "200px" }}>
  <WalletSelector />
</Col>
...
```

10. Start the local server with `npm start` open app in the browser.

We now have a working Wallet connect button and a wallet selector modal. Feel free to play with it and connect a wallet with it.
