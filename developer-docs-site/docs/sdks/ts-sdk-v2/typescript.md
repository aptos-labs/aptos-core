---
title: "Typescript"
---

Aptos SDK is designed to be type-safe! Things to keep in mind:

- Types currently require using TypeScript `v5.2.2` or greater.
- Changes to types in this repository are considered non-breaking and are usually released as patch semver changes (otherwise every type enhancement would be a major version!).
- It is highly recommended that you lock your `@aptos-labs/ts-sdk` package version to a specific patch release and upgrade with the expectation that types may be fixed or upgraded between any release.

### Local types

The SDK exports types on the top level and defines and holds all types locally and not using any external type generator excluding for Indexer GraphQL schema that even then the SDK customizes the generated types to be more user friendly and understandable.

You can check the types the SDK supports and exports on the [typedoc site](https://aptos-labs.github.io/aptos-ts-sdk/) organized by SDK version
