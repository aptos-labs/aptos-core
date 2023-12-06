---
title: "Programmatic Upgradeable Module"
slug: "programmatic-upgradeable-module"
---

# Programmatic Upgradeable Module

This tutorial will go over two sections:
1. How to publish modules to a resource account.
2. How to upgrade modules in a resource account.

A [resource account](https://aptos.dev/move/move-on-aptos/resource-accounts/) is a developer feature used to manage resources independent of an account managed by a user,
specifically publishing modules and providing on-chain-only access control, e.g. signers.

Typically, a resource account is used for two main purposes:

- Store and isolate resources; a module creates a resource account just to host specific resources.
- Publish module as a standalone (resource) account, a building block in a decentralized design where no private keys can control the resource account. The ownership (SignerCap) can be kept in another module, such as governance.

The first step will go over section 1: **How to publish modules to a resource account.**

## 1. How to publish modules to a resource account

Before publishing the module, we want to create a resource account prior, so that the module then gets uploaded to the resource account. To do this, we run the command:

1. `aptos move create-resource-account-and-publish-package --address-name <your-address> --seed <seed>`
    1. Replace `<your-address>` with your account address.
    2. Replace `<seed>` with a seed that only you know. This is used as a custom input to generating the resource account address.

When running this command, this will **FIRST calculate** your resource account address. 
A prompt below will show:

```jsx
Do you want to publish this package under the resource account's address 
0x090ad1536fe5cfcb5632b3026f99f8415c55b69ce54b6f17ed8cd7edbcb5edfa? [yes/no]
```

- `0x090ad1536fe5cfcb5632b3026f99f8415c55b69ce54b6f17ed8cd7edbcb5edfa` is the resource account address calculated, note this down in your `Move.toml` file, under the `[addresses]` section.
- Add a new entry `resource_account` and add the address generated as the value. in `Move.toml`

```jsx
[addresses]
your_module = "0x1d3e5574728c3d0d544f5679229fac153c5f7d78b323751b41811c5ffbb21cfd"
resource_account = "0x090ad1536fe5cfcb5632b3026f99f8415c55b69ce54b6f17ed8cd7edbcb5edfa"
```

2. Now that you have the `resource_account` address generated, terminate the operation by pressing `control + c`. 
If you have placeholder addresses, .e.g: `_` as addresses, refer to: [Placeholder addresses](#addresses-with-empty-placeholders)


3. You will now need to change all the module contracts addresses with the `resource_account`. For example in your module, you may have `your_module` as your account address.

```move
module your_module::user_info {
	// Your code...
}
```

Simply replace this with

```move
module resource_account::user_info {
	// Your code...
}
```

1. Now run the command again: `aptos move create-resource-account-and-publish-package --address-name <your-address> --seed <seed>`. 
It will ask for permission to spend gas (octas) for the transaction to go through. Once confirmed, you should see a result of **success**.

Congratulations, you have deployed your module onto a resource account. To verify, 
check the [**Aptos explorer**](https://explorer.aptoslabs.com/) and search for your resource account address. 
Click on `Modules` tab and you should see the code/module published onto the account.
**[Example](https://explorer.aptoslabs.com/account/0x090ad1536fe5cfcb5632b3026f99f8415c55b69ce54b6f17ed8cd7edbcb5edfa/modules/code/user_info?network=devnet)**

## 2. Upgrade module in resource account

Now that we have published a module onto a resource account, the next step would be attempting to upgrade the module.

The Aptos blockchain natively supports different *upgrade policies*, which allow move developers to explicitly define the constraints around how their move code can be upgraded. The default policy is *backwards compatible*. This means that code upgrades are accepted only if they guarantee that no existing resource storage or public APIs are broken by the upgrade (including public functions). This compatibility checking is possible because of Move's strongly typed bytecode semantics.

More information: https://aptos.dev/move/book/package-upgrades/

### Note that we need to redeploy the module to make it upgradeable, we’ll go over the steps again below.

### Publish the upgradeable module

Change `Move.toml` to include the `upgrade_policy`, we’ll set this to `compatible`.

```json
[package]
name = "MyApp"
version = "0.0.1"
upgrade_policy = "compatible"
...
```

### 2.1. Code Changes

To make a module upgradeable, there are two functions which need to be implemented.

1. `init_module`
2. `upgrade`

The `init_module` is the constructor of the module, it is called by the Move VM when deploying a module. It’s important to note that the `signer` generated from the VM is only generated once.
We need to generate a `SignerCapability` from this, and store it. Here is an example of how to store this safely:

```move
module resource_account::user_info {

	struct Config has key {
	    owner: address,
	    signer_cap: SignerCapability,
	}
	
	const OWNER: address = @your_module;
	const RESOURCE_ACCOUNT: address = @resource_account;
	
	/// `resource_account` injected from Move VM
	fun init_module(resource_account: &signer) {
	    // Must create this struct on constructor level, as we don't get the signer back when we create a resource account.
	    // The signer_cap was created from creating the resource account prior to creating this contract.
	    let signer_cap = resource_account::retrieve_resource_account_cap(resource_account, OWNER);
	    move_to(resource_account, Config {
	        owner: OWNER,
	        signer_cap,
	    });
	}
}
```

- Here we retrieve the `signer_cap` which was generated when creating the resource account.
- We then create a `Config` resource, store the `signer_cap` and the original owner of the module. In this case, your original address. We’ll use this information to configure access permissions when upgrading the module.
- `Config` is then moved to the `resource_account` signer in global storage.

#### Code Changes - `upgrade`

   The second function is to define the `upgrade` function. This must be a `entry` function as it’s being called from a transaction. Here’s an example of the function with access control configured.


```move
module resource_account::user_info {

	const RESOURCE_ACCOUNT: address = @resource_account;
	
	public entry fun upgrade(
	        owner: &signer,
	        metadata_serialized: vector<u8>,
	        code: vector<vector<u8>>,
	    ) acquires Config {
	        // Get the config we sent in the `init_module` to the resource_account.
	        let config = borrow_global<Config>(RESOURCE_ACCOUNT);
	        assert!(config.owner == signer::address_of(owner), 1);
	
	        // The resourec account `signer` is needed to publish/upgrade the contract
	        let signer = account::create_signer_with_capability(&config.signer_cap);
	        code::publish_package_txn(&signer, metadata_serialized, code);
	    }
}
```

- In the `init_module`, we moved the `Config` to the resource account signer. In this function, we retrieve it, and validate that the calling user is the `owner` from the `Config` resource.
- We then generate a `signer` from the `signer_cap` from the `Config`, and pass this to `publish_package_txn`, which expects the resource account signer. This is the address that the code will upgrade the packages.

**Now that the two functions have been implemented, deploy the module to the resource account, 
following the same steps in section: [How to publish modules to a resource account](#how-to-publish-modules-to-a-resource-account)**

### 2.2. Upgrade the module

Now that the upgradeable package has been deployed _(make sure this has been done above)_, we can make upgrades to the module. Let’s add a function to the module.

Make sure the rest of the functions, data structures are kept the same. Remember we made the upgrade_policy as `compatible`. Add a new function below the upgrade function:

```move
public fun new_function_added(): String {
    string::utf8(b"new_function_added")
}
```

Once the code changes have been made, we need to generate the inputs for `metadata_serialized` and `code` for this `upgrade` entry function. To do this, run:

- `aptos move build-publish-payload --json-output-file upgrade.json`

If you have placeholder addresses `_` in your `Move.toml` file, run the command here: [Upgrade function payload section](#generate-upgrade-function-payload-with-new-function-added-in-upgraded-module)


This will create a json file `upgrade.json`, with the args of the `metadata_serialied` and `code`. Just need to adjust the `function_id` in the json file, change this to:

`<resource_account>::user_info::upgrade`

- Replace `<resource_account>` with your resource account address.

Your `upgrade.json` file should look like:

```json
{
  "function_id": "0x090ad1536fe5cfcb5632b3026f99f8415c55b69ce54b6f17ed8cd7edbcb5edfa::user_info::upgrade",
  "type_args": [],
  "args": [
    {
      "type": "hex",
      "value": "0x0e436f646544657..."
    },
    {
      "type": "hex",
      "value": [
        "0xa11ceb0b060000000a01000c020c1..."
      ]
    }
  ]
}
```

1. Now deploy the modified contract:

```zsh
aptos move run \
--json-file upgrade.json
```

Now check the explorer,

1. Search for your resource account address.
2. Click `Modules`

You should see the source code with the new function `new_function_added` added. It should look like
```move
    public entry fun upgrade(
        owner: &signer,
        metadata_serialized: vector<u8>,
        code: vector<vector<u8>>,
    ) acquires Config {
        // Get the config we sent in the `init_module` to resource_account
        let config = borrow_global<Config>(RESOURCE_ACCOUNT);
        assert!(config.owner == signer::address_of(owner), 1);

        // This is needed to publish/upgrade the contract
        let signer = account::create_signer_with_capability(&config.signer_cap);
        code::publish_package_txn(&signer, metadata_serialized, code);
    }

    public fun new_function_added(): String {
        string::utf8(b"new_function_added")
    }
```

This concludes the tutorial.

## 3. Appendix

### Addresses with Empty Placeholders
If your [addresses] have empty placeholders, e.g:

```json
[addresses]
your_module = "_"
resource_account = "_"
```

Run:

`aptos move create-resource-account-and-publish-package --address-name your_module=0x1c3e5574728c3d0d544f5679229fac153c5f7d78b323751b41860c5ffbb21cfd --seed random3 --named-addresses your_module=0x1c3e5574728c3d0d544f5679229fac153c5f7d78b323751b41860c5ffbb21cfd,resource_account=0x090ad1536fe5cfcb5632b3026f99f8415c55b69ce54b6f17ed8cd7edbcb5edfa`

### Generate `upgrade` function payload with new function added in upgraded module

`aptos move build-publish-payload --json-output-file upgrade.json --named-addresses your_module=0x1c3e5574728c3d0d544f5679229fac153c5f7d78b323751b41860c5ffbb21cfd,resource_account=0x090ad1536fe5cfcb5632b3026f99f8415c55b69ce54b6f17ed8cd7edbcb5edfa`

