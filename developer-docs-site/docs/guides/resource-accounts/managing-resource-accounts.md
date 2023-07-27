---
title: "Managing resource accounts"
id: "managing-resource-accounts"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Managing resource accounts

In this section we're going to explore the various mechanisms underlying resource accounts and how to utilize them to manage resources programmatically.

There are two distinct ways to manage a resource account:

1. The authentication key is rotated to a separate account that can control it manually by signing for it
2. The authentication key is rotated to **0x0** and is controlled programmatically with a [**SignerCapability**](./common-questions#whats-a-signercapability)

:::info
The `create_resource_account(...)` functions in [`account.move`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/account.move) and [`resource_account.move`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/resource_account.move) are different, with the `account.move` version creating a programmatically controlled resource account and the `resource_account.move` version creating a manually controlled resource account.
:::

If you'd like to better understand how the creation functions work, check out [how are resource accounts created.](../resource-accounts/common-questions#how-are-resource-accounts-created)

## Using a SignerCapability

The [**SignerCapability**](./common-questions#whats-a-signercapability) resource is the most crucial part of how resource accounts work.

When a SignerCapability is created and later retrieved, the authentication key of the resource account it was created for is rotated to **0x0**, which gives the [Move VM](../../reference/glossary/#move-virtual-machine-mvm) the capability to generate the resource account's signer from a SignerCapability. You can store this SignerCapability and retrieve it later to create an authorized signer for the resource account.

Here is a very basic example that demonstrates how you'd use a SignerCapability in a Move contract:

```rust
// Define a resource we can store the SignerCapability at
struct SignerCap has key {
    signer_cap: SignerCapability,
}

public entry fun store_signer_capability(creator: &signer) {
    // We move `SignerCap` to an account's resources. We can even move it to the resource account itself:
    let (resource_signer, signer_cap) = account::create_resource_account(creator, b"seed bytes");
    move_to(resource_signer, SignerCap {
        signer_cap,
    });
}

// Generate the resource account's signer with the SignerCapability
public entry fun sign_with_resource_account(creator: &signer) acquires SignerCap {
    let resource_address = account::create_resource_address(signer::address_of(creator), b"seed bytes");
    let signer_cap = borrow_global<SignerCap>(resource_account_address);
    let resource_signer = account::create_signer_with_capability(signer_cap);

    // Here we'd do something with the resource_signer that we can only do with its `signer` primitive
}
```
Utilizing a resource account in this way is the fundamental process for automating the generation and retrieval of resources on-chain.

If you're wondering how the **SignerCapability** permission model works, head over to [what's stopping someone from using my SignerCapability?](./common-questions#whats-stopping-someone-from-using-my-signercapability)

## Retrieving a SignerCapability

Say you create a resource account with one of the `resource_account.move` functions. The `SignerCapability` exists for the account but you need to retrieve it- you can do this by calling `retrieve_resource_account_cap`.

Here's an example of how you could achieve this:

```rust title="Retrieve and store a SignerCapability"
struct SignerCap has key {
    signer_cap: SignerCapability,
}

// `source_addr` is the address of the resource account creator
public entry fun retrieve_cap(resource_signer: &signer, source_addr: address) acquires SignerCap {
    // Retrieve the SignerCapability
    let signer_cap = resource_account::retrieve_resource_account_cap(resource_signer, source_addr);
    // Move it into the resource signer's account resources for use later
    move_to(resource_signer, SignerCap {
        signer_cap,
    });
}
```
:::warning
Make sure to store a retrieved `SignerCapability` somewhere because the `retrieve_resource_account_cap` function can only be called once, since the `SignerCapability` returned is ephemeral and will be dropped if not stored.

Without access to a `SignerCapability`, there is no way to generate a signature for the account, effectively locking you out of it forever.
:::

## Publishing modules with resource accounts

Publishing modules with resource account gives developers the ability to separate the logic and resources of their smart contracts from their normal user accounts. It also offers them the ability to publish immutable, open source contracts that other developers can use without fear of the contract being altered.

Below we detail the various ways to publish a module to a resource account.

### Publishing an immutable module with a resource account

One of the most common usages of resource accounts is publishing a module with them. This function in `resource_account.move` is called by a user account to create a resource account and publish a module with it.

```rust title="Helper function in resource_account.move to publish a package with a resource account"
public entry fun create_resource_account_and_publish_package(
    origin: &signer,
    seed: vector<u8>,
    metadata_serialized: vector<u8>,
    code: vector<vector<u8>>,
) acquires Container {
    let (resource, signer_cap) = account::create_resource_account(origin, seed);
    aptos_framework::code::publish_package_txn(&resource, metadata_serialized, code);
    rotate_account_authentication_key_and_store_capability(
        origin,
        resource,
        signer_cap,
        ZERO_AUTH_KEY,
    );
}
```

:::warning Immutable Contracts
By default, publishing a module with a resource account will result in an immutable module. This is because the **SignerCapability** is retrieved and dropped during the publishing process.
:::

If you'd like to publish an upgradeable module with a resource account, see the section below.

### Publishing an upgradeable module with a resource account

Publishing an upgradeable module with a resource account involves a few steps to setup the contract to prepare it for publication and then the actual process of publishing and upgrading. In this section we'll explain how it works first and then how to do it after.

The key to making an upgradeable contract with a resource account is to use Aptos Move's **init_module** function. The **init_module** function is a unique function that will **only** run the first time a contract is published.

It's always a private function with a single argument (the module publisher, in the form of `&signer`) with no return value.

```rust title="init_module function signature"
fun init_module(publisher: &signer) {
    // ...
}
```

We can use this function to retrieve the SignerCapability and store it somewhere for use later, but since the resource that stores SignerCapability is not accessible externally, we must write a function call to interface with the SignerCapability.

```rust title="Using init_module and publish_package in a contract to function as an interface for the developer"
module upgradeable_resource_account_package::package_manager {
    use aptos_framework::account::{Self, SignerCapability};
    use aptos_framework::resource_account;
    use aptos_std::code;
    use std::signer;

    /// You are not authorized to upgrade this module.
    const ENOT_AUTHORIZED: u64 = 0;

    // Declare our SignerCap struct to store our resource account's SignerCapability.
    struct SignerCap has key {
        signer_cap: SignerCapability,
    }

    fun init_module(resource_signer: &signer) {
        // Note that we must deploy the module with `deployer` as a named address, otherwise the contract can't find the resource account's owner.
        let signer_cap = resource_account::retrieve_resource_account_cap(resource_signer, @deployer);

        // We move the SignerCap to the resource account.
        // Note that this means the SignerCap resource is stored at the same address that the module is.
        move_to(resource_signer, SignerCap {
            signer_cap,
        });
    }

    // This function is integral to making the contract upgradeable. Without it, the SignerCapability is still stored, but
    // there is no way to actually use it to upgrade the module, making the module effectively immutable.
    public entry fun publish_package(
        deployer: &signer,
        package_metadata: vector<u8>,
        code: vector<vector<u8>>,
    ) acquires PermissionConfig {
        assert!(signer::address_of(deployer) == @deployer, error::permission_denied(ENOT_AUTHORIZED));
        code::publish_package_txn(&get_signer(), package_metadata, code);
    }
}
```

There are several things going on here to take note of:

1. We retrieve and store the SignerCapability in the `SignerCap` struct.
2. We add an `publish_package(...)` function that acts as an interface to the package publishing function. It allows the **deployer** of the module to upgrade it despite not being the direct owner of the module.
3. The **deployer** is the developer's account- the one that calls `create_resource_account_and_publish_package(...)` and owns the resource account but not the module itself. It is a named address here, signified with `@deployer`, so we must specify it as a named address upon publication.
4. We gate access to the `publish_package` function by asserting that the signer is the original `@deployer`.

Publishing this module from the Aptos CLI would look like this:

If you're looking for an example of how to do this, see the [publishing an upgradeable module example.](./publishing-an-upgradeable-module.md)
