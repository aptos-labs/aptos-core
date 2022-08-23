---
title: "Upgrading Code"
slug: "upgrading"
---
import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Upgrading Code

The Aptos chain supports _code upgrade_, which means already deployed Move
code can be replaced with newer versions. Code upgrade enables
code owners to evolve their contracts or frameworks under a stable, well-known
account address, which can than be referenced by other applications, inside or
outside the chain.

## How it Works

Code upgrade on the Aptos chain happens on [Move package]
(https://move-language.github.io/move/packages.html) level. A package
in Aptos Move specifies an _upgrade policy_ in the manifest:

```toml
[package]
name = "MyApp"
version = "0.0.1"
upgrade_policy = "compatible"
```

The given policy means that any upgrades to this package must be _downwards
compatible_ both regards storage and apis, conditions which can be checked
mechanical from the Move bytecode:

- For storage, all old struct declarations must be in exactly the same way
  in the new code. This ensures the existing state of storage is correctly
  interpreted by the new code. Indeed, new struct declarations can be added.
- Similar for apis: all public functions must have the same signature as  
  before, but new ones can be added.

Aptos checks compatibility at the time a [Move package](https://move-language.github.io/move/packages.html) is published via a
dedicated Aptos framework transaction. This transaction aborts if
compatibility is not satisfied as specified.

### Different Upgrade Policies

Besides the upgrade policy `compatible`, a package owner can also choose
`immutable` (disallowing any future upgrade), or `arbitrary` (performing no
checks at all). The policy of a given package can start as `arbitrary`,  
then move to `compatible`, and then move to `immutable` -- but it can never
move back. This gives a consumer of a Move package verifiable guarantees how
the code now and in the future evolves.

### Programmatic Upgrade

In general, Aptos offers over the Move module `aptos_framework::code` ways
to publish code from arbitray points in your smart contracts. However,
notice that code published in the current transaction cannot be executed
before that transaction ends.

The Aptos Framework itself, including all the chain administration logic, is
an example for programmatic upgrade. The framework is marked as `compatible`.
Upgrade happens via specific generated governance scripts. For more details,
see [Aptos Governance](governance.md). 
