---
title: "Typescript SDK"
slug: "typescript-sdk"
---

# Aptos Typescript SDK

Aptos provides an official Typescript SDK. The Typescript SDK receives the most attention from the Aptos Labs team and community, meaning thorough testing and active updates. It is available on [npmjs](https://www.npmjs.com/package/aptos) with the source code in the [aptos-core GitHub repository](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/typescript/sdk).

## Compatability note
You must make sure to use the correct SDK for the environment you are developing in:

- Devnet: Use the latest package on [npmjs](https://www.npmjs.com/package/aptos).
- Local testnet: Use the SDK from the matching commit. You can read more about this testing workflow [here](/guides/local-testnet-dev-flow).

Unless you have a particular reason, you should use the first development setup, devnet + package on npmjs. You can read more about different deployments [here](/nodes/aptos-deployments).
