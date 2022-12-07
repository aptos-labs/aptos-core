---
title: "Wallet Adapter For Wallets"
id: "wallet-adapter-for-wallets"
---

# Wallet Adapter For Wallet builders

Having your wallet plugin follows the wallet standard and part of the wallet adapter can help you be exposed to more dapps in the Aptos Ecosystem and provides your users the functionality they are looking for in a wallet.

The [wallet-adapter-plugin-template repo](https://github.com/aptos-labs/wallet-adapter-plugin-template) provides wallet builders a pre-made class with all required wallet functionality following the wallet standard for easy and fast development.

### Usage

- `git clone git@github.com:aptos-labs/wallet-adapter-plugin-template.git`
- Open `src/index.ts`
- Change all AptosWindow appearances to `<Your-Wallet-Name>Window`
- Change `AptosWalletName` to be `<Your-Wallet-Name>WalletName`
- Change `url` to match your website url
- Change `icon` to your wallet icon (pay attention to the required format)
- Change `window.aptos` to be `window.<your-wallet-name>`
  - Make sure the `Window Interface` has `<your-wallet-name>` as a key (instead of `aptos`)
- Open `__tests/index.test.tsx` and change `AptosWallet` to `<Your-Wallet-Name>Wallet`
- Run tests with `pnpm test` - all tests should pass

At this point, you have a ready wallet class with all required props and functions to integrate with the Aptos Wallet Adapter.

### Publish as a Package

Next step is to publish your wallet as a npm package so dapps can install it as a dependency.

Creating and publishing scoped public packages [https://docs.npmjs.com/creating-and-publishing-scoped-public-packages](https://docs.npmjs.com/creating-and-publishing-scoped-public-packages)

Creating and publishing unscoped public packages [https://docs.npmjs.com/creating-and-publishing-unscoped-public-packages](https://docs.npmjs.com/creating-and-publishing-unscoped-public-packages)

:::tip
If your wallet provides function that is not included, you should open a PR against aptos-wallet-adapter in the core package so it would support this functionality. You can take a look at the signTransaction on the wallet core package
:::

### Add your name to the wallets list

Once the package is published, you can create a PR against the [aptos-wallet-adapter](https://github.com/aptos-labs/aptos-wallet-adapter) package and add your wallet name as a url to the npm package to the [supported wallet list](https://github.com/aptos-labs/aptos-wallet-adapter#supported-wallet-packages) on the README file.
