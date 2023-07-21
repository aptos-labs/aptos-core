# Package Upgrades

Move code (e.g., Move modules) on the Aptos blockchain can be upgraded. This
allows code owners and module developers to update and evolve their contracts
under a single, stable, well-known account address that doesn't change. If a
module upgrade happens, all consumers of that module will automatically receive
the latest version of the code (e.g., the next time they interact with it).

The Aptos blockchain natively supports different _upgrade policies_, which allow
move developers to explicitly define the constraints around how their move code
can be upgraded. The default policy is _backwards compatible_. This means that
code upgrades are accepted only if they guarantee that no existing resource storage
or public APIs are broken by the upgrade (including public functions).
This compatibility checking is possible because of Move's strongly typed bytecode
semantics.

We note, however, that even compatible upgrades can have hazardous effects on
applications and dependent Move code (for example, if the semantics of the underlying
module are modified). As a result, developers should be careful when depending on
third-party Move code that can be upgraded on-chain. See
[Security considerations for dependencies](#security-considerations-for-dependencies)
for more details.

## How it works

Move code upgrades on the Aptos blockchain happen at the [Move package](./packages.md)
granularity. A package specifies an upgrade policy in the `Move.toml` manifest:

```toml
[package]
name = "MyApp"
version = "0.0.1"
upgrade_policy = "compatible"
...
```
:::tip Compatibility check
Aptos checks compatibility at the time a Move package is published via an Aptos transaction. This transaction will abort if deemed incompatible.
:::

## How to upgrade

To upgrade already published Move code, simply attempt to republish the code at
the same address that it was previously published. This can be done by following the
instructions for code compilation and publishing using the
[Aptos CLI](../../tools/aptos-cli/use-cli/use-aptos-cli.md). For an example,
see the [Your First Move Module](../../tutorials/first-move-module.md) tutorial.

## Upgrade policies

There are two different upgrade policies currently supported by Aptos:

- `compatible`: these upgrades must be backwards compatible, specifically:
  - For storage, all old struct declarations must be the same in
    the new code. This ensures that the existing state of storage is 
    correctly interpreted by the new code. However, new struct declarations 
    can be added.
  - For APIs, all existing public functions must have the same signature as 
    before. New functions, including public and entry functions, can be added.
- `immutable`: the code is not upgradeable and is guaranteed to stay the same 
  forever.

Those policies are ordered regarding strength such that `compatible < immutable`,
i.e., compatible is weaker than immutable. The policy of a package on-chain can
only get stronger, not weaker. Moreover, the policy of all dependencies of a
package must be stronger or equal to the policy of the given package. For example,
an `immutable` package cannot refer directly or indirectly to a `compatible` package.
This gives users the guarantee that no unexpected updates can happen under the hood.

Note that there is one exception to the above rule: framework packages
installed at addresses `0x1` to `0xa` are exempted from the dependency check.
This is necessary so one can define an `immutable` package based on the standard
libraries, which have the `compatible` policy to allow critical upgrades and fixes.

## Compatibility rules
When using `compatible` upgrade policy, a module package can be upgraded. However, updates to existing modules already
published previously need to be compatible and follow the rules below:
- All existing structs' fields cannot be updated. This means no new fields can be added and existing fields cannot be
modified. Struct abilities also cannot be changed (no new ones added or existing removed).
- All public and entry functions cannot change their signature (argument types, type argument, return types). However,
argument names can change.
- Public(friend) functions are treated as private and thus their signature can arbitrarily change. This is safe as
only modules in the same package can call friend functions anyway and they need to be updated if the signature changes.

When updating your modules, if you see an incompatible error, make sure to check the above rules and fix any violations.

## Security considerations for dependencies

As mentioned above, even compatible upgrades can have disastrous effects for
applications that depend on the upgraded code. These effects can come from bugs,
but they can also be the result of malicious upgrades. For example,
an upgraded dependency can suddenly make all functions abort, breaking the
operation of your Move code. Alternatively, an upgraded dependency can make
all functions suddenly cost much more gas to execute then before the upgrade.
As result, dependencies to upgradeable packages need to be handled with care:

- The safest dependency is, of course, an `immutable` package. This guarantees
  that the dependency will never change, including its transitive dependencies.
  In order to update an immutable package, the owner would have to introduce a
  new major version, which is practically like deploying a new, separate
  and independent package. This is because major versioning can be expressed
  only by name (e.g. `module feature_v1` and `module feature_v2`). However,
  not all package owners like to publish their code as `immutable`, because this
  takes away the ability to fix bugs and update the code in place.
- If you have a dependency to a `compatible` package, it is highly 
  recommended you know and understand the entity publishing the package. 
  The highest level of assurance is when the package is governed by a
  Decentralized Autonomous Organization (DAO) where no single user can initiate
  an upgrade; a vote or similar has to be taken. This is the case for the Aptos
  framework.

## Programmatic upgrade

In general, Aptos offers, via the Move module `aptos_framework::code`, 
ways to publish code from anywhere in your smart contracts. However,
notice that code published in the current transaction can be executed 
only after that transaction ends.

The Aptos framework itself, including all the on-chain administration logic, is
an example for programmatic upgrade. The framework is marked as `compatible`.
Upgrades happen via specific generated governance scripts. For more details,
see [Aptos Governance](../../concepts/governance.md).
