---
title: "Upgrading Move Code"
slug: "upgrading-move-code"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Upgrading Move Code

The Aptos chain supports _code upgrade_, which means already deployed Move
code can be replaced with newer code. If a code upgrade happens, all 
consumers of the upgraded code will automatically receive the new code the
next time their code is executed. 

Code upgrade enables code owners to evolve their contracts or frameworks under
a stable, well-known account address, which can than be referenced by other
applications, inside or outside the chain.

Code upgrade is based on an _upgrade policy_ which the owner of a package
determines. The default policy is _(backwards) compatible_. That means, only
those upgrades are accepted which guarantee that no existing public APIs 
(including transactions and public functions) 
and/or existing resource storage is broken by the upgrade. This compatibility
checking is technical complete and possible because of Move's strongly typed 
byte code semantics. However, even a compatible upgrade can have 
hazardous effects on applications so depending on upgradable code on chain 
should be carefully considered on a case-by-case basis (see also below
discussion of security aspects).

## How it works

Code upgrade on the Aptos chain happens on [Move package](https://move-language.github.io/move/packages.html) 
level. A package specifies an upgrade policy in the `Move.toml`
manifest:

```toml
[package]
name = "MyApp"
version = "0.0.1"
upgrade_policy = "compatible"
...
```
:::tip Compatibility check
Aptos checks compatibility at the time a [Move package](https://move-language.github.io/move/packages.html) is published via a dedicated Aptos framework transaction. This transaction aborts if compatibility is not satisfied as specified.
:::

## Upgrade Policies

Currently, three different upgrade policies are supported:

- `compatible`: upgrades must be backwards compatible, specifically:
  - For storage, all old struct declarations must be the same in
    the new code. This ensures that the existing state of storage is 
    correctly interpreted by the new code. However, new struct declarations 
    can be added.
  - For APIs, all public functions must have the same signature as 
    before. New functions can be added.
- `immutable`: the code is not upgradable and guaranteed to stay the same 
  forever.

Those policies are ordered regarding strength such that `compatible < immutable`.
The policy of a package on chain can only get stronger, not weaker. Moreover,
the policy of all dependencies of a package must be stronger or equal to
the policy of the given package. For example, an `immutable` package
cannot refer directly or indirectly to a `compatible` package. This gives
users the guarantee that no unexpected updates happen
under the hood. There is one exception to the above rule: framework packages
installed at addresses `0x1` to `0xa` are exempted from the dependency check.
This is necessary so one can define an `immutable` package based on the standard
libraries, which have the `compatible` policy.

## Security considerations for dependencies

As mentioned, even compatible upgrades can lead to hazardous effects for
contracts depending on that upgraded code. Those effects can come simply
from bugs but can be also be the result of malicious upgrades. For example, an
upgraded dependency can suddenly make all functions
abort, breaking operation of your contract, or suddenly cost much more
gas to execute then before the upgrade. Because you cannot control
the upgrade, dependencies to upgradable packages need to be handled with
care.

- The safest dependency is, of course, to an `immutable` package. This is 
  guaranteed to never change, including its transitive dependencies (modulo 
  the Aptos framework). In order to evolve an immutable package, the owner 
  would have to introduce a new major version, which is practically like an 
  independent new package. Currently, major versioning would have to be 
  expressed by naming (e.g. `module feature_v1` and `module feature_v2`).
  However, not all package owners like to publish their code
  as `immutable`, because this takes away the ability to fix bugs and evolve 
  the code in place.
- If you have a dependency to a `compatible` package it is highly 
  recommended that you know and understand the entity publishing the package. 
  The highest level of assurance is that the package is governed by a DAO where 
  no single user can initiate an upgrade, but a vote or similar has 
  to be taken. This is for example the case for the Aptos framework.
   
## Programmatic upgrade

In general, Aptos offers, via the Move module `aptos_framework::code`, 
ways to publish code from anywhere in your smart contracts. However,
notice that code published in the current transaction cannot be executed 
before that transaction ends.

The Aptos Framework itself, including all the chain administration logic, is
an example for programmatic upgrade. The framework is marked as `compatible`.
Upgrade happens via specific generated governance scripts. For more details,
see [Aptos Governance](/concepts/governance.md).
