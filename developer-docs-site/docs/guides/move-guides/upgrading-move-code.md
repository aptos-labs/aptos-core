---
title: "Upgrading Move Code"
slug: "upgrading-move-code"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Upgrading Move Code

The Aptos chain supports _code upgrade_, which means already deployed Move
code can be replaced with newer versions. Code upgrade enables
code owners to evolve their contracts or frameworks under a stable, well-known
account address, which can than be referenced by other applications, inside or
outside the chain.

## How it works

Code upgrade on the Aptos chain happens on [Move package](https://move-language.github.io/move/packages.html) level. A package in Aptos Move specifies an _upgrade policy_ in the manifest:

```toml
[package]
name = "MyApp"
version = "0.0.1"
upgrade_policy = "compatible"
```

The above `upgrade_policy = "compatible"` policy means that any upgrades to this package must be _downwards compatible_ both in regards to storage and APIs, conditions which can be checked mechanical from the Move bytecode:

- For storage, all old struct declarations must be in exactly the same way
  in the new code. This ensures that the existing state of storage is correctly
  interpreted by the new code. Also, new struct declarations can be added.
- Similarly for APIs, all public functions must have the same signature as before, but new ones can be added.

:::tip Compatibility check
Aptos checks compatibility at the time a [Move package](https://move-language.github.io/move/packages.html) is published via a dedicated Aptos framework transaction. This transaction aborts if compatibility is not satisfied as specified.
:::

## Upgrade policies

The upgrade policy is specified using the key `upgrade_policy`. In addition to the above-shown `upgrade_policy = "compatible"` policy, the following policies are supported:

- `immutable`: A package owner can also choose `immutable`, to disallow any future upgrade.
- `arbitrary`: Perform no checks at all. The policy of a given package can start as `arbitrary`, then move to `compatible`, and then move to `immutable`, **but it can never move back**. This gives a consumer of a Move package verifiable guarantees on how the code is now and how it evolves in the future.

## Programmatic upgrade

In general, Aptos offers, via the Move module `aptos_framework::code`, ways to publish code from arbitray points in your smart contracts. However, notice that code published in the current transaction cannot be executed before that transaction ends.

The Aptos Framework itself, including all the chain administration logic, is
an example for programmatic upgrade. The framework is marked as `compatible`.
Upgrade happens via specific generated governance scripts. For more details,
see [Aptos Governance](/concepts/governance.md).
