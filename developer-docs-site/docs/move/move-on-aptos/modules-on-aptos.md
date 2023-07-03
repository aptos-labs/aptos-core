---
title: "Modules on Aptos"
slug: "modules-on-aptos"
---

# Modules on Aptos

Aptos allows for permissionless publishing of [modules](../book/modules-and-scripts.md) within a [package](../book/packages.md) as well as [upgrading](../book/package-upgrades.md) those that have appropriate compatibility policy set.

A module contains several structs and functions, much like Rust.

During package publishing time, a few constraints are maintained:
* Both Structs and public function signatures are published as immutable.
* Only when a module is being published for the first time, and not during an upgrade, will the VM search for and execute an `init_module(account: &signer)` function. The signer of the account that is publishing the module is passed into the `init_module` function of the contract.  **This function must be private and not return any value.** 

:::tip `init_module` is optional
It is only necessary if you want to initialize data when publishing a module for the first time.
:::
