---
title: "Utilizing resource accounts"
id: "utilizing-resource-accounts"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Utilizing Resource Accounts

In this guide we're going to show you how you can use resource accounts to automate smart contracts and manage resources programmatically. If you're looking for an introduction to resource accounts, head to the [conceptual understanding](../resource-accounts/understanding-resource-accounts) section.

There are two distinct ways to manage a resource account:

1. The authentication key is rotated to a separate account that can control it manually by signing for it
2. The authentication key is rotated to **0x0** and is controlled programmatically with a [**SignerCapability**](./understanding-resource-accounts#whats-a-signercapability)

:::info
The `create_resource_account(...)` functions in [`account.move`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/account.move) and [`resource_account.move`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/resource_account.move) are different, with the `account.move` version creating a programmatically controlled resource account and the `resource_account.move` version creating a manually controlled resource account.
:::

If you'd like to better understand how the creation functions work, check out [how are resource accounts created.](../resource-accounts/understanding-resource-accounts#how-are-resource-accounts-created)

## Using a SignerCapability

The [**SignerCapability**](./understanding-resource-accounts#whats-a-signercapability) resource is the most crucial part of how resource accounts work.

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

If you're wondering how the **SignerCapability** permission model works, head over to [what's stopping someone from using my SignerCapability?](./understanding-resource-accounts#whats-stopping-someone-from-using-my-signercapability)

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

## Publishing a module to a resource account

One of the most common usages of resource accounts is publishing a module with them. This function in `resource_account.move` is called by a user account to create a resource account and publish a module with it.

```rust title="Helper function in resource_account.move to publish a package to a resource account"
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
By default, publishing a module to a resource account will result in an immutable contract. This is because the **SignerCapability** is retrieved and dropped during the publishing process.
:::

If you'd like to publish an upgradeable module to a resource account, see the section below.

## Publishing an upgradeable module with a resource account

Publishing an upgradeable module with a resource account involves a few steps to setup the contract to prepare it for publication and then the actual process of publishing and upgrading. In this section we'll explain how it works first and then how to do it after.

### How it works

The key to making an upgradeable contract with a resource account is to use Aptos Move's **init_module** function. The **init_module** function is a unique function that will **only** run the first time a contract is published.

It's always a private function with a single argument (the module publisher, in the form of `&signer`) with no return value.

```rust title="init_module function signature"
fun init_module(publisher: &signer) {
    // ...
}
```

We can use this function to retrieve the SignerCapability and store it somewhere for use later:

```rust title="Using init_module to store a publishing resource account's SignerCapability"
module upgradeable_resource_contract::package_manager {
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

    // NOTE: This function is integral to making the contract upgradeable. Without it, the SignerCapability is stored, but
    // there is no way to actually use it to upgrade the module.
    public entry fun upgrade_module(
        deployer: &signer,
        package_metadata: vector<u8>,
        code: vector<vector<u8>>,
    ) acquires SignerCap {
        // NOTE: If we leave this line out, anyone can upgrade the contract and potentially hijack its functionality.
        assert!(signer::address_of(deployer) == @deployer, error::permission_denied(ENOT_AUTHORIZED));
        let signer_cap = &borrow_global<SignerCap>(@upgrade_resource_contract).signer_cap;
        let resource_signer = account::create_signer_with_capability(signer_cap);
        code::publish_package_txn(&resource_signer, package_metadata, code);
    }
}
```

There are several things going on here to take note of:

1. We retrieve and store the SignerCapability in the `SignerCap` struct.
2. We add an `upgrade_module(...)` function that acts as an interface to the package publishing function. It allows the **deployer** of the module to upgrade it despite not being the direct owner of the module. 
3. The **deployer** is the developer's account- the one that calls `create_resource_account_and_publish_package(...)` and owns the resource account but not the module itself. It is a named address here, signified with `@deployer`, so we must specify it as a named address upon publication.
4. We gate access to the `upgrade_module` function by asserting that the signer is the original `@deployer`.

Publishing this module from the Aptos CLI would look like this:

```shell
aptos move create-resource-account-and-publish-package           \
                --address-name upgrade_resource_contract         \
                --named-addresses owner=CONTRACT_DEPLOYER        \
                --profile CONTRACT_DEPLOYER
```

Where `CONTRACT_DEPLOYER` is the profile. Read more about [Aptos CLI profiles here.](../../tools/aptos-cli/use-cli/use-aptos-cli#creating-other-profiles)

### Step-by-step guide

Let's run through an example of how to publish the above upgradeable contract to a resource account and upgrade it.

1. Publish the module to a resource account
2. Run the `upgradeable_function` view function and see what it returns
3. Upgrade the module using the json output from the `aptos move build-publish-package` command
4. Run the `upgradeable_function` view function again to see the new return value

First make sure you have a default profile initialized to devnet.

```shell
aptos init --profile default
```

Choose `devnet` and leave the private key part empty so it will generate an account for you. When we write `default` in our commands, it will automatically use this profile.

Navigate to the `move-examples/upgrade_resource_contract` directory.

### Publish the module

```shell
aptos move create-resource-account-and-publish-package --address-name upgrade_resource_contract --seed '' --named-addresses owner=default
```

The `--address-name` flag denotes that the resource address created from the resource account we make will be supplied as the `upgrade_resource_contract` address in our module. Since we declared it as the module address with `module upgrade_resource_contract::upgrader { ... }` at the very top of our contract, this is where our contract will be deployed.

When you run this command, it will ask you something like this:

```
Do you want to publish this package under the resource account's address be326762ddd27624743223991c2223027621e62b7d0849a40a970fa2df385da9? [yes/no] >
```

Say yes and copy that address to your clipboard. That's our resource account address where the contract is deployed.

Now you can run the view function!

### Run the view function

```shell
aptos move view --function-id RESOURCE_ACCOUNT_ADDRESS::upgrader::upgradeable_function
```

Remember to replace `RESOURCE_ACCOUNT_ADDRESS` with the resource account address you deployed your module to; it is different from the one posted above, so this will specifically only work for *your* contract.

It should output:
```json
Result: [
    9000
]
```

### Change the view function

Now let's change the value returned in the view function from `9000` to `9001` so we can see that we've upgraded the contract:

```rust
#[view]
public fun upgradeable_function(): u64 {
    9001
}
```

Save that file, and then use the `build-publish-package` command to get the bytecode output in JSON format.

### Get the bytecode for the module

```shell
aptos move build-publish-payload --json-output-file upgrade_contract.json --named-addresses upgrade_resource_contract=RESOURCE_ACCOUNT_ADDRESS,owner=default
```

Replace `RESOURCE_ACCOUNT_ADDRESS` with your resource account address and run the command. Once you do this, there will now be a `upgrade_contract.json` file with the bytecode output of the new, upgraded module in it.

The hex values in this JSON file are arguments that we'd normally use to pass into the `0x1::code::publish_package_txn` function, but since we made our own `upgrade_contract` function that wraps it, we need to change the function call value to something else.

Your JSON should look something like the below output, just with expanded `value` fields (truncated here for simplicity's sake):

```json
{
  "function_id": "0x1::code::publish_package_txn",
  "type_args": [],
  "args": [
    {
      "type": "hex",
      "value": "0x2155...6200"
    },
    {
      "type": "hex",
      "value": [
        "0xa11c...0000"
      ]
    }
  ]
}
```

Change the `function_id` value in the JSON file to match your contract's upgrade function contract, with your resource account address filled in:

```
"function_id": "RESOURCE_ACCOUNT_ADDRESS::upgrader::upgrade_contract",
```

Save this file so we can use it to run an entry function with JSON parameters.

### Run the upgrade_contract function

```shell
aptos move run --json-file upgrade_contract.json
```

Confirm yes to publish the upgraded module where the view function will return 9001 instead of 9000.

### Run the upgraded view function

```shell
aptos move view --function-id RESOURCE_ACCOUNT_ADDRESS::upgrader::upgradeable_function
```

You should get:

```json
Result: [
    9001
]
```

Now you know how to publish an upgradeable module to a resource account!

## Creating and funding a resource account

Another common usage is to create and fund a resource account, in case the account needs access to functions that need access to `Coin<AptosCoin>`:

```rust
// resource_account.move
public entry fun create_resource_account_and_fund(
    origin: &signer,
    seed: vector<u8>,
    optional_auth_key: vector<u8>,
    fund_amount: u64,
) acquires Container {
    let (resource, signer_cap) = account::create_resource_account(origin, seed);
    coin::register<AptosCoin>(&resource);
    coin::transfer<AptosCoin>(origin, signer::address_of(&resource), fund_amount);
    rotate_account_authentication_key_and_store_capability(
        origin,
        resource,
        signer_cap,
        optional_auth_key,
    );
}
```
