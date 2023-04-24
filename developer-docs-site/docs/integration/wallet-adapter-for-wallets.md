---
title: "For Wallets"
id: "wallet-adapter-for-wallets"
---

# Wallet Adapter For Wallet Builders

To gain from dapps in the Aptos Ecosystem and provide your users the functionality they are looking for in a wallet, your wallet plugin should follow the [Aptos Wallet Standard](../standards/wallets.md) and be built from the Aptos Wallet Adapter.

The [wallet-adapter-plugin-template](https://github.com/aptos-labs/wallet-adapter-plugin-template) repository gives wallet builders a pre-made class with all required wallet functionality following the Aptos Wallet Standard for easy and fast development.

## Configuration

1. `git clone git@github.com:aptos-labs/wallet-adapter-plugin-template.git`
1. Open `src/index.ts` for editing.
1. Replace all `AptosWindow` references with: `<Your-Wallet-Name>Window`
1. Replace `AptosWalletName` with: `<Your-Wallet-Name>WalletName`
1. Replace `url` with your website URL.
1. Change `icon` to your wallet icon (pay attention to the required format).
1. Replace `window.aptos` with: `window.<your-wallet-name>`
  - Make sure the `Window Interface` has `<your-wallet-name>` as a key (instead of `aptos`).
1. Open `__tests/index.test.tsx` and change `AptosWallet` to: `<Your-Wallet-Name>Wallet`
1. Run tests with `pnpm test` - all tests should pass.

At this point, you have a ready wallet class with all required properties and functions to integrate with the Aptos Wallet Adapter.

### Publish as a package

The next step is to publish your wallet as an NPM package so dapps can install it as a dependency. Use one of the options below:

[Creating and publishing scoped public packages](https://docs.npmjs.com/creating-and-publishing-scoped-public-packages)

[Creating and publishing unscoped public packages](https://docs.npmjs.com/creating-and-publishing-unscoped-public-packages)

:::tip
If your wallet provides functionality that is not included, you should open a pull request against `aptos-wallet-adapter` in the core package to have it support this functionality. See the `signTransaction` on the [wallet core package](https://github.com/aptos-labs/aptos-wallet-adapter/blob/main/packages/wallet-adapter-core/src/WalletCore.ts) for guidance.
:::

### Add your name to the wallets list

Once the package is published, create a pull request against the [aptos-wallet-adapter](https://github.com/aptos-labs/aptos-wallet-adapter) package and add your wallet name to the [supported wallet list](https://github.com/aptos-labs/aptos-wallet-adapter#supported-wallet-packages) on the README file as a URL to your NPM package.
