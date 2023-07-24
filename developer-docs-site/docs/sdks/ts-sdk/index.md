---
title: "TypeScript Index"
slug: "index"
hidden: false
---

# Aptos TypeScript SDK

Aptos provides a fully supported TypeScript SDK with the source code in the [Aptos-core GitHub](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/typescript/sdk) repository. Much of the functionality of the TypeScript SDK can be found in the [Rust](../rust-sdk.md) and [Python](../python-sdk.md) SDKs. Nevertheless, Aptos strongly encourages you to use the TypeScript SDK for app development whenever possible.

## Installing the TypeScript SDK

1. Make sure you [downloaded the latest precompiled binary for the Aptos CLI](../../tools/aptos-cli/install-cli/index.md#download-precompiled-binary).
2. On a terminal run the below command to install the TypeScript SDK from [npmjs](https://www.npmjs.com/package/aptos):

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

## Using the TypeScript SDK

See the [Developer Tutorials](../../tutorials/index.md) for code examples showing how to use the Typescript SDK.

## TypeScript SDK Architecture

See the [TypeScript SDK Architecture](./typescript-sdk-overview.md) for the components that make up the TypeScript SDK.

## Additional information

- ### [TypeScript SDK Source code](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/typescript/sdk)
- ### [TypeScript SDK at NPM](https://www.npmjs.com/package/aptos)
- ### [TypeScript SDK Reference](https://aptos-labs.github.io/ts-sdk-doc/)
- ### [TypeScript SDK Reference Source](https://github.com/aptos-labs/ts-sdk-doc)
