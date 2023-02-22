---
title: "TypeScript Index"
slug: "index"
hidden: false
---

# Aptos Typescript SDK

Aptos provides a fully supported TypeScript SDK with the source code in the [Aptos-core GitHub](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/typescript/sdk) repository. Much of the functionality of the Typescript SDK can be found in the [Rust](../rust-sdk.md) and [Python](../python-sdk.md) SDKs. Nevertheless, Aptos strongly encourages you to use the Typescript SDK for app development whenever possible.

## Installing the Typescript SDK

1. Make sure you [downloaded the latest precompiled binary for the Aptos CLI](../../cli-tools/aptos-cli-tool/install-aptos-cli.md#download-precompiled-binary). On a terminal run the below command to install the Typescript SDK from [npmjs](https://www.npmjs.com/package/aptos):
   ```bash
   npm i aptos
   ```

   or

   ```bash
   yarn add aptos
   ```

   or

   ```bash
   pnpm add aptos
   ```

   :::tip
   The above command installs the TS SDK and should not be confused as installing the Aptos CLI.
   :::

## Using the Typescript SDK

See the [Developer Tutorials](../../tutorials/index.md) for code examples showing how to use the Typescript SDK.

## Typescript SDK Architecture

See the [Typescript SDK Architecture](./typescript-sdk-overview.md) for the components that make up the Typescript SDK.

## Additional information
- ### [TypeScript SDK Reference](https://aptos-labs.github.io/ts-sdk-doc/)
- ### [TypeScript SDK Reference Source](https://github.com/aptos-labs/ts-sdk-doc)
- ### [TypeScript SDK at NPM](https://www.npmjs.com/package/aptos)
- ### [TypeScript SDK Source](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/typescript/sdk)
