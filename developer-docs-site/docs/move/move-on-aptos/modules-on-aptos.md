---
title: "Modules on Aptos"
slug: "modules-on-aptos"
---

# Modules on Aptos

Aptos allows for permissionless publishing of [modules](../book/modules-and-scripts.md) within a [package](../book/packages.md) as well as [upgrading](../book/package-upgrades.md) those that have appropriate compatibility policy set.

A module contains several structs and functions, much like Rust.

During package publishing time, a few constraints are maintained:
* Both Structs and public function signatures are published as immutable.
* When publishing a module for the first time, The VM will search for and execute an `init_module(account: &signer)` function.
