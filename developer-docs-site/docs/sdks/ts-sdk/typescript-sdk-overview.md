---
title: "Typescript SDK Architecture"
slug: "typescript-sdk-overview"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

This document describes the main features and components of the Aptos TypeScript SDK.

The [Aptos TypeScript SDK](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/typescript/sdk) provides APIs and interfaces you can use to interact with the Aptos blockchain for reading the blockchain state and for sending your transaction to the Aptos blockchain.

# TypeScript SDK Architecture

The Aptos TypeScript SDK has three logical layers:

1. Transport layer - Responsible on communication with the blockchain server.
2. Core SDK layer - Exposes the functionalities needed by most applications.
3. Plugins layer - Implementation of different use cases such as Token, NFT, ANS, etc

See below a high-level architecture diagram of the Aptos TypeScript SDK.

<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/ts-sdk-light.svg'),
    dark: useBaseUrl('/img/docs/ts-sdk-dark.svg'),
  }}
/>

## Components of the TypeScript SDK

- [Transport Layer](./sdk-transport-layer.md)
- [Core Layer](./sdk-core-layer.md)
- [Plugins Layer](./sdk-plugins-layer.md)

- [Validation](./sdk-validation.md)
