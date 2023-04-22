---
title: "3. Add Wallet Support"
id: "add-wallet-support"
---

# 3. Add Wallet Support

In the third chapter of the tutorial on [building an end-to-end dapp on Aptos](./index.md), you will be adding _wallet_ support to your [React app](./2-set-up-react-app.md). You now need a wallet to submit a transaction to the blockchain.

Aptos provides a [wallet adapter](../../integration/wallet-adapter-concept.md) that supports many ecosystem wallets to offering a common interface and UI package that can be used to add a wallet connect button and a wallet selector modal.

1. Stop the local server if running.
2. In the `client` folder, run:

```cmd
npm i @aptos-labs/wallet-adapter-react@1.0.2
```

```cmd
npm i @aptos-labs/wallet-adapter-ant-design@1.0.0
```

This installs two packages:

- the adapter React provider that holds the logic.
- a wallet connect UI package.

3. We now need to add wallets to our app. There is a list of [wallets the adapter supports](https://github.com/aptos-labs/aptos-wallet-adapter#supported-wallet-packages); but to keep this tutorial simple, we will use only one wallet.
   Still in the `client` folder, run

```cmd
npm i petra-plugin-wallet-adapter
```

:::tip
If you haven't installed the Petra wallet extension yet:

1. [Install Petra Wallet](https://petra.app) and open the Chrome extension.
2. Follow the [user instructions](https://petra.app/docs/use) on petra.app for help.
3. Switch to the Devnet network by clicking **Settings** > **Network** and selecting **devnet**.
4. Click the **Faucet** button to ensure you can receive test tokens.

:::

4. Open `Index.tsx` file. At the top of the file, add the following:

```js
import { PetraWallet } from "petra-plugin-wallet-adapter";
import { AptosWalletAdapterProvider } from "@aptos-labs/wallet-adapter-react";
```

5. Still in `Index.tsx`, add a constant that holds an array of wallets:

```js
...
const wallets = [new PetraWallet()];
...
```

6. Inside the `render` method, update the code with the following:

```jsx
...
<AptosWalletAdapterProvider plugins={wallets} autoConnect={true}>
  <App />
</AptosWalletAdapterProvider>
...
```

That wraps our app with the adapter provider and initializes it with our wallets. It also sets the provider to autoConnect a wallet.

7. Open the `App.tsx` file and import the wallet connect UI package we installed in the previous step. At the top of the file add the following:

```js
import { WalletSelector } from "@aptos-labs/wallet-adapter-ant-design";
```

8. The UI package uses a style `.css` file; let's import that one also at the bottom of the import statements.

```js
...
import "@aptos-labs/wallet-adapter-ant-design/dist/index.css";
```

9. In the `return` statement, remove the `<h1>Connect Wallet</h1>` text and add the `WalletSelector` component:

```jsx
...
<Col span={12} style={{ textAlign: "right", paddingRight: "200px" }}>
  <WalletSelector />
</Col>
...
```

10. Start the local server with `npm start` and open the app in the browser.

We now have a working Wallet connect button and a wallet selector modal. Feel free to play with it and connect a wallet with it.

Then learn how to [fetch data from chain](./4-fetch-data-from-chain.md) in chapter 4.
