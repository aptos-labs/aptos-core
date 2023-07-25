---
title: "Publishing an upgradeable module"
id: "publishing-an-upgradeable-module"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Publishing an upgradeable module

The objective of this tutorial is to show a concrete example of how to publish an upgradeable module with a resource account.

In order to follow along with this tutorial, make sure you have:

* The [Aptos CLI](../../tools/aptos-cli)
* The [Aptos core repository.](https:/github.com/aptos-labs/aptos-core) The modules we'll use are located in the `aptos-move/move-examples` section of the repository.

## Overview of the publishing process

Each of the following steps will be explained in detail in their corresponding sections:

1. Publish the module with a resource account
2. Run the `upgradeable_function` view function and see what it returns
3. Change the view function in the contract so we can observe a tangible change post-publish
4. Get the module metadata and bytecode from the `aptos move build-publish-package` command
5. Run the `publish_package` function to upgrade the module
6. Run the view function again to observe the new return value

First navigate to the correct directory:
```shell title="Navigate to your local directory"
cd ~/aptos-core/aptos-move/move-examples/upgradeable_resource_account_package
```

Then create a `deployer` profile initialized to devnet:
```shell
aptos init --profile deployer
```

Enter devnet when prompted and leave the private key empty so it will generate an account for you. When we write `deployer` in our commands, it will automatically use this profile.

### 1. Publish the module

```shell
aptos move create-resource-account-and-publish-package                      \
           --address-name upgradeable_resource_account_package --seed ''    \
           --named-addresses deployer=deployer                              \
           --profile deployer
```

The `--address-name` flag marks the following string as named address the resource account's address will appear as in the contract.

That is, the resource account created from this command will correspond to the `@upgradeable_resource_account_package` address in our module.

When you run this command, it will ask you something like this:

```
Do you want to publish this package under the resource account's address 
be326762ddd27624743223991c2223027621e62b7d0849a40a970fa2df385da9?
[yes/no] >
```

Enter yes and copy that address down somewhere. That's our resource account address where the contract is deployed. Now you can run the view function!

### 2. Run the view function

Replace `RESOURCE_ACCOUNT_ADDRESS`` with the address you got from step #1 and run the following command:

```shell title="View the value returned from upgradeable_function()"
aptos move view --function-id RESOURCE_ACCOUNT_ADDRESS::basic_contract::upgradeable_function --profile deployer
```

It should output:
```json
Result: [
    9000
]
```

### 3. Change the view function

Now let's change the value returned in the view function from `9000` to `9001` so we can observe a difference in the upgraded contract:

```rust
#[view]
public fun upgradeable_function(): u64 {
    9001
}
```

Save that file, and then follow step #4 to get the package metadata and bytecode in JSON format.

### 4. Get the new bytecode for the module

```shell
aptos move build-publish-payload --json-output-file upgrade_contract.json --named-addresses upgradeable_resource_account_package=RESOURCE_ACCOUNT_ADDRESS,deployer=deployer
```

Replace `RESOURCE_ACCOUNT_ADDRESS` with your resource account address and run the command. Once you do this, there will now be a `upgrade_contract.json` file with the bytecode output of the new, upgraded module in it.

Since we made our own `upgrade_contract` function that wraps the `0x1::code::publish_package_txn`, we need to change the function call value to our publish package function.

After editing `upgrade_contract.json`, your `function_id` value should look something like below:

```json title="Change the function_id value in upgrade_contract.json to call your resource account's publish package function"
{
  "function_id": "RESOURCE_ACCOUNT_ADDRESS::package_manager::publish_package",
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

Make sure to change the `RESOURCE_ACCOUNT_ADDRESS` to your specific resource account address.

### 5. Run the upgrade_contract function

```shell
aptos move run --json-file upgrade_contract.json --profile deployer
```

Confirm yes to publish the upgraded module.

### 6. Run the upgraded view function

```shell
aptos move view --function-id RESOURCE_ACCOUNT_ADDRESS::basic_contract::upgradeable_function --profile deployer
```

You should get:

```json
Result: [
    9001
]
```

Now you know how to publish an upgradeable module with a resource account!

## Extra features in the package manager module

You may have noticed there are other features in the `package_manager.move` module. These are helper functions to manage retrieving the `signer` for a resource account used to publish a package module and additionally track addresses generated from the contract in a `SmartTable`.

The `get_signer()` function utilizes the friend keyword to gate access to the signer.
```rust title="The get_signer() function can only be called by friends or other functions in package_manager.move"
public(friend) fun get_signer(): signer acquires PermissionConfig {
    let signer_cap = &borrow_global<PermissionConfig>(@upgradeable_resource_account_package).signer_cap;
    account::create_signer_with_capability(signer_cap)
}
```

If you wanted to give another one of your modules access to this function this function, you'd simply declare the module as a friend at the top of your module, and then call it in that module wherever you need:

```rust
// package_manager.move
module upgradeable_resource_account_package::package_manager {
    friend upgradeable_resource_account_package::basic_contract;
    // ...
}

// basic_contract.move
module upgradeable_resource_account_package::basic_contract {
    use upgradeable_resource_account_package::package_manager;
    // ...
    public fun move_to_resource_account(deployer: &signer) {
        // Only the deployer can call this function.
        assert!(signer::address_of(deployer) == @deployer, error::permission_denied(ENOT_AUTHORIZED));

        // Do something with the resource account's signer
        // For example, a simple `move_to` call
        let resource_signer = package_manager::get_signer();
        move_to(
            &resource_signer,
            SomeResource {
                value: 42,
            }
        );
    }
}
```
Read more about [friend function declarations here.](../../move/book/friends/#friend-declaration)

Use the package manager contract as needed in your own packages. Keep in mind that you must deploy the `package_manager.move` module with your package in order to correctly utilize the module.
