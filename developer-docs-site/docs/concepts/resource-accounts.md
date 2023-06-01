---
title: "Resource Accounts"
id: "resource-accounts"
---

## What is a resource account?

A resource account is an [Account](https://aptos.dev/concepts/accounts/) that's used to store and manage resources. It can be a simple storage account that's used merely to separate different resources for an account or a module, or it can be utilized to programmatically control resource management in a contract.

There are two distinct ways to manage a resource account. In this guide we'll discuss how to implement each technique, variations on the implementations, and any configuration details relevant to the creation process.

## How to utilize a resource account

### Manually controlling a resource account by rotating its auth key to another account's auth key

The first technique we're going to discuss is through the `create_resource_account` function in the `resource_account.move` contract. View the code [here](https://github.com/aptos-labs/aptos-core/blob/4beb914a168bd358ec375bfd9854cffaa271199a/aptos-move/framework/aptos-framework/sources/account.move#L602).

You can specify a `seed` byte vector and an optional auth key to rotate the resulting resource account's auth key to.
```rust
public entry fun create_resource_account(
    origin: &signer,
    seed: vector<u8>,
    optional_auth_key: vector<u8>,
) acquires Container {
    let (resource, resource_signer_cap) = account::create_resource_account(origin, seed);
    rotate_account_authentication_key_and_store_capability(
        origin,
        resource,
        resource_signer_cap,
        optional_auth_key,
    );
}
```
When you create a resource account like this, the account with the matching auth key can sign for it. Transactions signed in this way will show the signer/sender account as the resource account, but it was actually signed by the owning account with the matching auth key. This is mostly for separating resources into a different account- it's merely a way to organize and manage resources.

Notice that there is nothing returned here- we are not given anything to store or manage. We simply created a resource account and rotated its auth key to the optional auth key.

:::tip
If you don't specify an auth key, that is, if you pass in `vector::empty<u8>()` or `vector<u8> []` to the `optional_auth_key` field, it will automatically rotate the auth key to the `origin` account's auth key.
:::

### Managing a resource account programmatically with a SignerCapability

The second technique is the `create_resource_account` function in the `account.move` contract. View the code [here](https://github.com/aptos-labs/aptos-core/blob/4beb914a168bd358ec375bfd9854cffaa271199a/aptos-move/framework/aptos-framework/sources/account.move#L602).

```rust
public fun create_resource_account(
    source: &signer,
    seed: vector<u8>
): (signer, SignerCapability) acquires Account {
    // ...
}
```

When this function is called, the auth key of the resource account is rotated to `0x0`, which gives the Move VM the capability to generate the resource account's signer from a `SignerCapability`. You can store this `SignerCapability` and retrieve it later to sign for the resource account.

This is often integral to automating a smart contract in Move. It gives the developer the ability to generate a signer for an account programmatically.

Notice that the creation function returns the resource account's `signer` and a `SignerCapability` resource. Let's discuss what a `SignerCapability` is and then how to store it.

#### What's a SignerCapability?

A SignerCapability is a very simple resource, mostly meant to abstractly represent the ability to sign for an account. It doesn't actually *do* anything special, but its existence somewhere implies that if you have access to it, you either created it or received access to it very intentionally.

It contains a single field called `account` the matches the address it's intended to generate a signature for:

```rust
struct SignerCapability has drop, store {
    account: address
}
```

Since it only has the abilities `drop` and `store`, it can't be copied, meaning only `account.move` itself can manage the new creation of a `SignerCapability`. The inner `account` field cannot be altered post creation, so it can only sign for the resource account it was initially created for.

Here is a very basic example that demonstrates how you'd use a `SignerCapability` in a Move contract:

```rust
// define a resource we can store the SignerCapability in. We use key here for simplicity's sake
struct MySignerCapability has key {
    resource_signer_cap: SignerCapability,
}

public entry fun store_signer_capability(creator: &signer) {
    // We store `MySignerCapability` to an account's resources. We can even store it on the resource account itself:
    let (resource_signer, resource_signer_cap) = account::create_resource_account(creator, b"seed bytes");
    move_to(resource_signer, MySignerCapability {
        resource_signer_cap,
    });
}

// Now we utilize the resource account by generating its signer with the SignerCapability
public entry fun sign_with_resource_account(creator: &signer) acquires MySignerCapability {
    let resource_address = account::create_resource_address(signer::address_of(creator), b"seed bytes");
    let signer_cap = borrow_global<MySignerCapability>(resource_account_address);
    let resource_signer = account::create_signer_with_capability(signer_cap);

    // here we'd do something with the resource_signer that we can only do with its `signer`, like transfer coins, create/transfer an NFT, or call some other function that rqeuires a signer.
    // be careful with making functions like these entry functions. If you have no contingencies for a function like this, they can be very easily abused.
}
```
Utilizing a resource account in this way is the fundamental process for automating the generation and retrieval of resources on-chain.

You might be wondering "*Why does this work? Isn't it dangerous to be able to create a signer for an account so easily?*"

Yes, you need to make sure you're gating access to a `SignerCapability` whenever you store it somewhere. Be very thoughtful with how you facilitate access to one, because unrestricted access to it gives free reign for anyone to call any function that requires a signer with it.

:::tip
To intuitively understand why a `SignerCapability` is allowed to be so powerful, you need to consider how resource storage and control work in Move. You can't directly access, create, or modify a resource outside of the module it's defined in, meaning if you have access to a resource in some way, the creator of the module it belongs to explicitly gave it to you.

Upon creating the `SignerCapability`, you're free to decide how you want to expose it. You can store it somewhere, give it away, or gate its access to functions that use it or conditionally return it.
:::

### Using a resource account to publish a module

There are a few other ways we can utilize a resource account. One common usage is to use it to publish a module:

```rust
// resource_account.move
public entry fun create_resource_account_and_publish_package(
    origin: &signer,
    seed: vector<u8>,
    metadata_serialized: vector<u8>,
    code: vector<vector<u8>>,
) acquires Container {
    let (resource, resource_signer_cap) = account::create_resource_account(origin, seed);
    aptos_framework::code::publish_package_txn(&resource, metadata_serialized, code);
    rotate_account_authentication_key_and_store_capability(
        origin,
        resource,
        resource_signer_cap,
        ZERO_AUTH_KEY,
    );
}
```

:::warning Immutable Contracts
By default, publishing a module to a resource account means it will be immutable *unless* you store the SignerCapability somewhere in the `init_module` function. This is because the auth key is rotated to `ZERO_AUTH_KEY`, meaning the only way to control it is through a `SignerCapability`.

If you don't store the `SignerCapability` there is no way to retrieve the resource account's signer, rendering it immutable.

You *also* need to provide some way to use or retrieve the `SignerCapability`, too, or you won't even be able to use it.
:::

### Publishing an upgradeable module with a resource account

If you want to publish to a resource account and also have an upgradeable contract, use the `init_module` function to use the resource account's signer to retrieve and store the `SignerCapability`. Here's a full working example:

```rust
module upgrade_resource_contract::upgrader {
    use std::signer;
    use std::account::{SignerCapability};
    use std::resource_account;
    use std::account;
    use std::code;

    struct MySignerCapability has key {
        resource_signer_cap: SignerCapability,
    }

    fun init_module(resource_signer: &signer) {
        assert!(signer::address_of(resource_signer) == @upgrade_resource_contract, 0);
        let resource_signer_cap = resource_account::retrieve_resource_account_cap(resource_signer, @owner);
        move_to(resource_signer, MySignerCapability {
            resource_signer_cap: resource_signer_cap,
        });
    }

    // Note the assertion that the caller is @owner. If we leave this line out, anyone can upgrade the contract, exposing the resource account's resources and the contract functionality.
    public entry fun upgrade_contract(
        owner: &signer,
        metadata_serialized: vector<u8>,
        code: vector<vector<u8>>,
    ) acquires MySignerCapability {
        assert!(signer::address_of(owner) == @owner, 1);
        let resource_signer_cap = &borrow_global<MySignerCapability>(@upgrade_resource_contract).resource_signer_cap;
        let resource_signer = account::create_signer_with_capability(resource_signer_cap);
        code::publish_package_txn(
            &resource_signer,
            metadata_serialized,
            code,
        );
    }
}
```

The `init_module` function is a special function that is called a single time upon the initial publication of a module. It inherently passes in the caller's `&signer`, which in our case is the resource account. This gives us a brief opportunity to store the `SignerCapability` somewhere.

The `upgrade_contract` function takes in the owner as a signer and then borrows the resource signer cap, generates the resource account's signer, and publishes the package code from the input. Keep in mind you need to serialize the data for these two arguments correctly, or it won't work.

Also note that the `retrieve_resource_account_cap` function takes in the source address as its second argument, so you need to somehow pass in the account address being used to create and publish. In our case, we used the named address `@owner` and specify it with an Aptos CLI profile:

```shell
aptos move create-resource-account-and-publish-package --address-name upgrade_resource_contract --named-addresses owner=CONTRACT_DEPLOYER --profile CONTRACT_DEPLOYER
```

Where `CONTRACT_DEPLOYER` is the profile. Read more about [Aptos CLI profiles here](https://aptos.dev/tools/aptos-cli-tool/use-aptos-cli/#creating-other-profiles).

If you want to see an end to end unit test displaying how to publish and then upgrade the code above by calling `upgrade_contract`, you can run the cargo test:

```shell
cargo test --package e2e-move-tests --lib -- tests::upgrade_resource_contract::code_upgrading_using_resource_account --exact --nocapture
```

### Creating and funding a resource account

Another common usage is to create and fund a resource account, in case the account needs access to functions that need access to `Coin<AptosCoin>`:

```rust
// resource_account.move
public entry fun create_resource_account_and_fund(
    origin: &signer,
    seed: vector<u8>,
    optional_auth_key: vector<u8>,
    fund_amount: u64,
) acquires Container {
    let (resource, resource_signer_cap) = account::create_resource_account(origin, seed);
    coin::register<AptosCoin>(&resource);
    coin::transfer<AptosCoin>(origin, signer::address_of(&resource), fund_amount);
    rotate_account_authentication_key_and_store_capability(
        origin,
        resource,
        resource_signer_cap,
        optional_auth_key,
    );
}
```

#### Can I acquire a SignerCapability later?

Yes. Say you create a resource account and rotate its auth key to your account's auth key. You'd just need to sign for the account and call `retrieve_resource_account_cap` in order to get the `SignerCapability` and store it somewhere:

```rust
struct MySignerCapability has key {
    resource_signer_cap: SignerCapability,
}

public entry fun retrieve_cap(resource_signer: &signer, source_addr: address): acquires MySignerCapability {
    let resource_signer_cap = resource_account::retrieve_resource_account_cap(resource_signer, source_addr);
    move_to(resource_signer, MySignerCapability {
        resource_signer_cap,
    });
}
```

Call the function, but change the sender account to appear as the resource account with the CLI flag `--sender-account`. If the source address is the `default` profile:

```shell
aptos move run --function-id MODULE_ADDRESS::MODULE_NAME::retrieve_cap --args address:default --sender-account RESOURCE_ADDRESS_HERE --profile default
```

#### How is the address for a resource account derived?

When a resource account is created, the address is derived from a SHA3-256 hash of the requesting account's address plus an optional byte vector `seed`. If you want to know the resource address generated by an account + a given arbitrary seed, you can call the `create_resource_address` function in `account.move`:

```rust
account::create_resource_address(your_account_address, seed);
```

You can view the resource account functionality in more detail at [account.move](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/account.move) and [resource_account.move](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/resource_account.move).