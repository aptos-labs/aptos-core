---
title: "Mint NFTs (v2) with the Aptos CLI"
---

# Mint NFTs (v2) with the Aptos CLI

This tutorial is intended to demonstrate how to programmatically mint NFTs on Aptos. The simplest version of a minting contract is the NFT collection creator manually minting and sending an NFT to a user.

We build upon this several times until we eventually create an automated NFT minting smart contract that has:
- Token metadata for each token minted, and...
- An allowlist that manages:
  - The start time of the event
  - The end time
  - The price to mint one token
  - Multiple tiers that for each of the above details

## Prerequisites

This tutorial assumes you have:

* the [Aptos CLI](../../tools/install-cli/index.md) (or you can run from [aptos-core](https://github.com/aptos-labs/aptos-core) source via `cargo run`)
* the `aptos-core` repository checked out: `git clone https://github.com/aptos-labs/aptos-core.git`
* a basic understanding of Move, NFTs and NFT Collections
* installed `jq` a CLI tool to parse JSON values

## 0. Set up your CLI profile

To make things simple, let's initialize a profile and export its account address and the corresponding contract address to our environment variables, so we can easily re-use them later.

```shell
echo '' | aptos init --profile mint_deployer --network devnet --assume-yes
echo '' | aptos init --profile nft_minter --network devnet --assume-yes
export MINT_DEPLOYER=0x(aptos account lookup-address --profile mint_deployer | jq -r ".Result")
export NFT_MINTER=0x(aptos account lookup-address --profile nft_minter | jq -r ".Result")
export RESOURCE_ACCOUNT_ADDRESS=0x(aptos account derive-resource-account-address --address $MINT_DEPLOYER --seed '' --seed-encoding utf8 | jq -r ".Result")
echo "Mint deployer address    => $MINT_DEPLOYER"
echo "Resource account address => $RESOURCE_ACCOUNT_ADDRESS"
echo "NFT minter address       => $NFT_MINTER"
```

If you didn't install `jq`, replace `jq -r ".Result"` with `grep "Result" | cut -d'"' -f4`.

## 1. Creating a simple smart contract to mint an NFT

We're going to start by making the simplest form of the flow for creating a collection and minting a token and sending it to a user. The code for this part of the tutorial is in the first section of the `move-examples/mint_nft_v2_part1` folder in the aptos-core repository.

Here are the things we need to do first:

* Create the NFT collection and store the configuration options for it
* Mint a non-fungible token within that collection using the configuration options
* Send the minted token to a user account

### Defining the configuration options

The first thing we need to do is store the fields necessary to identify the collection and mint a token from it. We store these so we don't have to pass them in as fields to our mint function later.

```rust
// This struct stores all the relevant NFT collection and token's metadata
struct Metadata has key {
    collection_name: String,
    creator: address,
    token_description: String,
    token_name: String,
    token_uri: String,
    property_keys: vector<String>,
    property_types: vector<String>,
    property_values: vector<vector<u8>>,
}
```

We give `Metadata` the `key` ability so that the module contract can store it as a global resource at an address. This means we can retrieve it in the contract later if we know of an address with the `Metadata` resource.

We set this data to the deploying account in our `init_module` function, which is set when we first publish the module.

In our `init_module` function we have the `aptos_token::create_collection` function, which is what creates the collection with the `deployer` as the creator, with all our configuration options passed in:
```rust
aptos_token::create_collection(
    deployer,
    description,
    maximum_supply,
    collection_name,
    collection_uri,
    false, // mutable_description
    false, // mutable_royalty
    false, // mutable_uri
    false, // mutable_token_description
    false, // mutable_token_name
    true, // mutable_token_properties
    false, // mutable_token_uri
    false, // tokens_burnable_by_creator
    false, // tokens_freezable_by_creator
    5, // royalty_numerator
    100, // royalty_denominator
);
```

```rust title="Create the Metadata resource and move it to the deployer account"
let metadata = Metadata {
    collection_name,
    creator: deployer_address,
    token_description: string::utf8(b""),
    token_name,
    token_uri,
    property_keys: vector<String>[string::utf8(b"given_to")],
    property_types: vector<String>[ string::utf8(b"address") ],
    property_values: vector<vector<u8>>[bcs::to_bytes(&deployer_address)],
};
move_to(deployer, metadata);
```

### Writing a simple mint function

First off, note the first few lines of the `mint_to` function. We assert that the caller here is the same as the account that owns the module:

```rust title="Ensure only the deployer can call this function"
public entry fun mint_to(
    deployer: &signer,
    receiver_address: address
) acquires Metadata {
    let deployer_address = signer::address_of(deployer);
    assert!(deployer_address == @no_code_mint_p1, error::permission_denied(ENOT_AUTHORIZED));
    // ...
}
```

:::note
We define `@no_code_mint_p1` when we deploy the module with the `--named-addresses no_code_mint_p1=$MINT_DEPLOYER` flag.
:::

We then borrow the `Metadata` resource from where we stored it in `init_module` and create the token with it:

```rust title="Populate the mint_token_object function with the data from our Metadata resource and transfer it to the designated receiver"
let metadata = borrow_global_mut<Metadata>(@no_code_mint_p1);
        // mint the token object
        let token_object = aptos_token::mint_token_object(
            deployer,
            metadata.collection_name,
            metadata.token_description,
            metadata.token_name,
            metadata.token_uri,
            metadata.property_keys,
            metadata.property_types,
            metadata.property_values,
        );

        // transfer it to the receiver
        object::transfer(deployer, token_object, receiver_address);
```

```rust title="Update the PropertyMap metadata on the token
// Remember that prior to this, the `color` property on the token was a string "BLUE", taken from our original `Metadata` resource:
property_keys: vector<String>[string::utf8(b"color")],
property_types: vector<String>[ string::utf8(b"string") ],
property_values: vector<vector<u8>>[bcs::to_bytes(&string::utf8(b"BLUE"))],

// we passed this to `mint_token_object`, so it has an initial property "color": "BLUE"
// we then update it with:
aptos_token::update_property(
  deployer,
  token_object,
  string::utf8(b"color"),
  string::utf8(b"string"),
  bcs::to_bytes(&string::utf8(b"RED")),
);
```
:::tip Advanced Info
Property maps are unique polymorphic data structures that enable storing multiple data types into a mapped vector. You can read more about them in the [aptos-token-objects/property_map.move](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token-objects/sources/property_map.move) contract.
:::

Great! We now have a very simple module that initializes a collection and can mint a token to an account. Let's run it in the next section.

### Publishing the module

Navigate to the `no_code_mint/1-Create-NFT` folder and let's publish our module:

```rust title="Publish the module"
aptos move publish --named-addresses no_code_mint_p1=$MINT_DEPLOYER
                   --assume-yes
```

```tip title="Move.toml"
The `Move.toml` file in the top directory for a Move contract shoould have a Move.toml file. This file specifies logistical things like the package name, dependencies and named addresses.

Here is where we declare that we want to specify our `no_code_mint_p1` named address in the contract when we publish the contract with the `--named-addresses no_code_mint_p1=$MINT_DEPLOYER` flag. This lets us use @no_code_mint_p1 in our *.move files as an address.
```

### Running the contract

To call a Move entry function, you need to provide:
1. The address it was published to
2. The module name
3. The function name
4. The function parameters

You can find these at the top of a module. In our case, we have:

```rust title="Our module address and module name"
module no_code_mint_p1::create_nft {
  // ...
}
```

1. Module address: `@no_code_mint_p1`, which we stored as our shell variable `$MINT_DEPLOYER`
2. Module name: `create_nft`
3. Function name: `mint_to`
4. Function parameters: `deployer: &signer` and `receiver_address: address`

We don't need to provide any `&signer` arguments, so we only need to provide a single argument, the `receiver_address`.

In our case, that's just the `$NFT_MINTER` address.

```shell title="Construct the function call with the Aptos CLI"
aptos move run --function-id $MINT_DEPLOYER::create_nft::mint_to
               --profile mint_deployer
               --args address: $NFT_MINTER
               --assume-yes
```

Note that `$MINT_DEPLOYER` can be used interchangeably with `mint_deployer` in the CLI, because `mint_deployer` is our named Aptos CLI profile.

:::info title="Why don't you need to provide a &signer argument?"
When you sign and submit a transaction with an account's private key, you automatically pass the first `&signer` parameter to the function.

Running an entry function with `--profile mint_deployer` signs and submits the transaction for the `mint_deployer` profile, which is why you don't need to provide the signer to the `--args` parameter list.
:::

Congratulations! You've created a collection, minted an NFT, and transferred the NFT to another account.

To view the events in this transaction, paste the transaction hash in the Aptos explorer search bar and navigate to the events section, or directly go to:

https://explorer.aptoslabs.com/txn/YOUR_TRANSACTION_HASH_HERE/events?network=devnet

You should see a `0x4::collection::MintEvent` and a `0x1::object::TransferEvent`.

## 2. Automating the mint function with a resource account

The issue with the code we've written so far is that it requires explicit approval from the creator to mint a token. The process isn't automated and the receiver doesn't ever approve of receiving the token.

The first step to improving the flow of this process is automating the creator's approval. We can do this with the use of what's called a resource account.

To achieve this, in this section we'll show you how to:

- Create the NFT collection with a resource account
- Store the capability to sign things with the resource account, a `SignerCapability`, into the owner's resources on-chain
- Automate minting the token to the user; that is, write a mint function that works without the collection creator's signature

### What is a resource account?

A resource account is essentially an account that another account can own. They are useful for separating and managing different types of resources, but they're also capable of delegating decisions to sign transactions to the logic in a smart contract.

If you want to approve a transaction for later, but don't want to have to be present to sign the transaction, you can write Move code to manage the conditional signature from a resource account to approve that transaction. You can view the resource account functionality in [account.move](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/account.move) and [resource_account.move](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/resource_account.move).

### Adding a resource account to our contract

Most of the code for our contract in this second part is very similar, so we're only going to discuss the parts added that make it different.

:::note
Please note that in this contract, the resource account will now technically be the `creator` of the collection, so for clarity we've changed the account representing the deployer (you) to be named `owner` and the account that creates the collection and mints tokens to remain `creator.`
:::

Let's start by adding the `SignerCapability` to our contract, which is the structure that produces the capability to sign a transaction programmatically.

We'll store it in our `MintConfiguration`:

```rust
struct MintConfiguration has key {
    signer_capability: SignerCapability,
    collection_name: String,
    token_name: String,
    token_uri: String,
}
```

We create it, providing it a seed in the form of the collection name.

```rust
let seed = *string::bytes(&collection_name);
let (resource_signer, resource_signer_cap) = account::create_resource_account(owner, seed);
```

:::info
The seed can be anything we want, but since resource accounts are unique hashes of the combination of the creating account + seed, it's good to make the seed something that will also be unique. In our case, the owner and collection name combination will always be unique because that is a constraint enforced by the `collection.move` contract, so the seed being the collection name logically follows.
:::

To clarify, the `resource_signer` is the actual structure on chain that signs things, it is of type `signer`; whereas the `SignerCapability` is a unique
on-chain resource that generates a signer for an account, given whomever requesting it has permission to access the `SignerCapability` resource.

Now we can provide the `resource_signer` as the creator of the collection, and move the `resource_signer_cap` to the `MintConfiguration`, so we can programmatically retrieve the creator's ability to sign later.

Note also that we store the `MintConfiguration` resource onto the resource account now, so when you call `create_collection` you'll have to look up the resource account's address to call the mint function later.

```rust
aptos_token::create_collection(
    &resource_signer,
    // ...
);

move_to(&resource_signer, MintConfiguration {
    signer_capability: resource_signer_cap,
    collection_name,
    token_name,
    token_uri,
});
```

Let's alter the mint function so that it uses the resource account instead of the owner account.

The first thing to notice is that the arguments to the function have changed. We no longer need the owner or the creator to sign the transaction. To do this before, we would've had to implement a function that takes two signers, which would've been complex. Not requiring the signer, however, meant the receiver had no say in whether or not they even wanted to receive the NFT.

Now, we can require the receiver to sign so that a user can mint whenever they like, and the owner doesn't have to approve of it beforehand.

```rust
public entry fun mint(receiver: &signer, resource_addr: address) acquires MintConfiguration {
    //...
}
```

Note that we require the user to pass in the resource address- we've provided a view function as one way for you to calculate it. We show you how to use this function later in the [Running the contract section](#running-the-contract-1).

```rust
#[view]
public fun get_resource_address(collection_name: String): address {
    account::create_resource_address(&@mint_nft_v2_part2, *string::bytes(&collection_name))
}
```

:::tip Advanced Tip
Computing the `resource_addr` inside the `mint` function with `account::create_resource_address(...)` has heavy computational overhead because it uses a cryptographic hashing function. In some instances where we only call the `mint` function a few times, this might be okay, but since a `mint` function is intended to be called by potentially thousands of users in a very short period of time, we ensure that it has been precomputed and have the user pass it in as an argument.
:::

Next, we access the mint configuration data to retrieve the signer capability. We generate a temporary signer with `account::create_signer_with_capability` and use it to sign the mint function and transfer the token object to the receiver.

```rust
public entry fun mint(receiver: &signer) acquires MintConfiguration {
    // access the configuration resources stored on-chain at @mint_nft_v2_part2's address
    let mint_configuration = borrow_global<MintConfiguration>(@mint_nft_v2_part2);
    let signer_cap = &mint_configuration.signer_capability;
    let resource_signer: &signer = &account::create_signer_with_capability(signer_cap);
    // ...
    // ... similar code as part 1
    // ... just replace `creator` and `creator_addr` with `resource_signer` and `resource_addr`
    // ...
}
```
:::warning
Be careful about how you generate and retrieve signers from a `SignerCapability` resource. It is, in essence, the keys to a resource account. If you purposely or inadvertently let any account access a `SignerCapability` freely, they can do almost anything they want with the resources in the associated account.

In our case, our code essentially makes the mint free, because there is no cost to mint and anyone can do it as many times as they like. This could be
intentional in some cases, but should be considered before hand. Always be highly aware of how you grant access to a resource account's signer capability.
:::

### Publishing the module and running the contract

Publishing the module is basically the same as before. Just make sure you're in the `2-Using-Resource-Account` directory and run this command, note the only thing that changed is the module name in the first line, `create_nft_with_resource_account` instead of `create_nft`:

```shell
aptos move publish --named-addresses mint_nft_v2_part2=default --profile default --assume-yes
```

Call this function as the owner of the contract, which is our `default` profile. Keep in mind the `--profile default` flag:

```shell
aptos move run --function-id default::create_nft_with_resource_account::initialize_collection   \
               --profile default                                          \
               --args                                                     \
                  string:"Krazy Kangaroos"                                \
                  string:"https://www.link-to-your-collection-image.com"  \
                  u64:3                                                   \
                  u64:5                                                   \
                  u64:100                                                 \
                  string:"Krazy Kangaroo #1"                              \
                  string:"https://www.link-to-your-token-image.com"       
```

Next we need to get the resource address for the contract with our view function.

```shell
aptos move view --function-id default::create_nft_with_resource_account::get_resource_address \
                --profile default \
                --args string:"Krazy Kangaroos"
```

Now we call this function as a user, which we simulate with our `nft-receiver` profile:

```shell
aptos move run --function-id default::create_nft_with_resource_account::mint \
               --profile nft-receiver \
               --args address:YOUR_RESOURCE_ADDRESS_HERE
```

Great! Now you've created the collection as an owner and requested to mint as a user and received the newly minted NFT.

It may not feel different since you're acting as the owner and the receiver all from the command line, but in an actual dapp this user flow makes much more sense than before.

In the first section, the user has to wait for the owner of the contract to mint and send them an NFT. In the second section, the user can request to mint and receive an NFT themselves.

## 3. Adding restrictions: a whitelist, an end time, an admin, and an enabled flag

We're still missing some very common features for NFT minting contracts:

1. A whitelist that restricts minting to whitelisted addresses
2. The ability to add/remove addresses from the whitelist
3. A start* and end time
4. The ability to enable or disable the mint
5. An admin model: restricting using these functions to an assigned admin account

*We add the start time in part 4 to keep this section brief.

### Adding the new configuration options

We need to add the expiration timestamp, the enabled flag, and the admin address to the mint configuration resource:

```rust
struct MintConfiguration has key {
    // ...
    whitelist: Table<address, bool>,
    expiration_timestamp: u64,
    minting_enabled: bool,
    admin: address,
}
```

Note that we're storing a `bool` in the whitelist as the value in each key: value pair. We won't use it in this tutorial, but you could easily use it to limit each account to 1 mint or even use an integer type to limit it to an arbitrary number of mints.  

When we initialize the collection, we create a default empty whitelist, an expiration timestamp that's one second in the past, and disable the mint:

```rust
public entry fun initialize_collection( /* ... */ ) {
    // ...

    move_to(&resource_signer, MintConfiguration {
        // ...
        whitelist: table::new<address, bool>(),
        expiration_timestamp: timestamp::now_seconds() - 1,
        minting_enabled: false,
        admin: owner_addr,
    });
}
```

### Using assertions to enforce rules

We can utilize these fields to enforce restrictions on the mint function by aborting the call with an error message if any of the conditions aren't met:

```rust
public entry fun mint(receiver: &signer, resource_addr: address) acquires MintConfiguration {
    // ...

    // abort if user is not in whitelist
    assert!(table::contains(&mint_configuration.whitelist, receiver_addr), ENOT_IN_WHITELIST);
    // abort if this function is called after the expiration_timestamp
    assert!(timestamp::now_seconds() < mint_configuration.expiration_timestamp, error::permission_denied(ECOLLECTION_EXPIRED));
    // abort if minting is disabled
    assert!(mint_configuration.minting_enabled, error::permission_denied(EMINTING_DISABLED));

    // ...
}
```

:::note
Function calls with failed assertions don't have side effects. When an error is thrown after a function alters a field with `borrow_global_mut`, none of the changes in the entire transaction occur. This includes any resource affected by nested and parent function calls.
:::

We also need a way to set all of these values, but we don't want to give just anyone the ability to freely set these fields. We can ensure that in our setter functions, the account requesting the change
is also the designated admin:

### Enabling the mint and setting the expiration time

```rust
public entry fun set_minting_enabled(
    admin: &signer,
    minting_enabled: bool,
    resource_addr: address,
) acquires MintConfiguration {
    let mint_configuration = borrow_global_mut<MintConfiguration>(resource_addr);
    let admin_addr = signer::address_of(admin);
    // abort if the signer is not the admin
    assert!(admin_addr == mint_configuration.admin, error::permission_denied(ENOT_AUTHORIZED));
    mint_configuration.minting_enabled = minting_enabled;
}
```

The `set_expiration_timestamp` function is almost identical to `set_minting_enabled`, so we've left it out.

### Setting the admin of the module

If we want to change the admin, we'll do something similar:

```rust
public entry fun set_admin(
    current_admin: &signer,
    new_admin_addr: address,
    resource_addr: address,
) acquires MintConfiguration {
    let mint_configuration = borrow_global_mut<MintConfiguration>(resource_addr);
    let current_admin_addr = signer::address_of(current_admin);
    // ensure the signer attempting to change the admin is the current admin
    assert!(current_admin_addr == mint_configuration.admin, error::permission_denied(ENOT_AUTHORIZED));
    // ensure the new admin address is an account that's been initialized so we don't accidentally lock ourselves out
    assert!(account::exists_at(new_admin_addr), error::not_found(ENOT_FOUND));
    mint_configuration.admin = new_admin_addr;
}
```
Note the extra error check to make sure the new admin account exists. If we don't check this, we could accidentally lock ourselves out by setting the admin to an account that doesn't exist yet.

### Adding to the whitelist

Now let's add our add_to_whitelist and remove_from_whitelist functions. They're very similar, so we'll just show the former:

```rust
public entry fun add_to_whitelist(
    admin: &signer,
    addresses: vector<address>,
    resource_addr: address
) acquires MintConfiguration {
    let admin_addr = signer::address_of(admin);
    let mint_configuration = borrow_global_mut<MintConfiguration>(resource_addr);
    assert!(admin_addr == mint_configuration.admin, error::permission_denied(ENOT_AUTHORIZED));

    vector::for_each(addresses, |user_addr| {
        // note that this will abort in `table` if the address exists already- use `upsert` to ignore this
        table::add(&mut mint_configuration.whitelist, user_addr, true);
    });
}
```

Most of this is fairly straightforward, although note the new inline function we use with `for_each`. This is a functional programming construct Aptos Move offers that lets us run an inline function over each element in a vector. `user_addr` is the locally named element that's passed into the `for_each` function block.

:::tip Why do we use a table instead of a vector for the whitelist?
You might be tempted to use a `vector<address>` for this, but the lookup time of a vector gets prohibitively expensive when the size of the list starts growing into the thousands.

A Table offers very efficient lookup times. Since it's a hashing function, it's an O(1) lookup time. A vector is O(n). When it comes to thousands of calls on-chain, that can make a substantial difference in execution cost and time.
:::


### Publishing the module and running the contract

Navigate to the `3-Adding-Admin` directory and publish the module for part 3:

```shell
aptos move publish --named-addresses mint_nft_v2_part3=default --profile default --assume-yes
```

Initialize the collection:

```shell
aptos move run --function-id default::create_nft_with_resource_and_admin_accounts::initialize_collection   \
               --profile default                                          \
               --args                                                     \
                  string:"Krazy Kangaroos"                                \
                  string:"https://www.link-to-your-collection-image.com"  \
                  u64:3                                                   \
                  u64:5                                                   \
                  u64:100                                                 \
                  string:"Krazy Kangaroo #1"                              \
                  string:"https://www.link-to-your-token-image.com"       
```

Get the new resource address:

```shell
aptos move view --function-id default::create_nft_with_resource_and_admin_accounts::get_resource_address \
                --profile default \
                --args string:"Krazy Kangaroos"
```

Mint as `nft-receiver`:

```shell
aptos move run --function-id default::create_nft_with_resource_and_admin_accounts::mint \
               --profile nft-receiver \
               --args address:YOUR_RESOURCE_ADDRESS_HERE
```

We haven't set our expiration timestamp to be in the future yet, so you should get an error here:

```shell
"ECOLLECTION_EXPIRED(0x50002): The collection minting is expired"
```

Okay, let's try to set the timestamp. Here's an easy way to get a current timestamp in seconds:

```shell
aptos move view --function-id 0x1::timestamp::now_seconds
```

Add enough time to this so you can mint before the timestamp expires.

```shell
aptos move run --function-id default::create_nft_with_resource_and_admin_accounts::set_expiration_timestamp \
               --profile default                           \
               --args                                      \
                   u64:YOUR_TIMESTAMP_IN_SECONDS_HERE      \
                   address:YOUR_RESOURCE_ADDRESS_HERE   
```

If you try to mint again, you should get a different error this time:

```shell
"EMINTING_DISABLED(0x50003): The collection minting is disabled"
```

Enable the mint:

```shell
aptos move run --function-id default::create_nft_with_resource_and_admin_accounts::set_minting_enabled \
               --profile default                           \
               --args                                      \
                   bool:true                               \
                   address:YOUR_RESOURCE_ADDRESS_HERE   
```

Last error we'll get is the user not being on the whitelist:

```shell
"ENOT_IN_WHITELIST(0x5): The user account is not in the whitelist"
```

Add the user to the whitelist:

```shell
aptos move run --function-id default::create_nft_with_resource_and_admin_accounts::add_to_whitelist \
               --profile default                           \
               --args                                      \
                   "vector<address>:nft-receiver"          \
                   address:YOUR_RESOURCE_ADDRESS_HERE
```

Try to mint again, and it should succeed! You can try setting the admin with the `set_admin(...)` call and then set the `whitelist`, `expiration_timestamp` and `minting_enabled` fields on your own. Use the correct and incorrect admin to see how it works.


## 4. Adding a public phase, custom events, and unit tests

We've got most of the basics down, but there are some additions we can still make to round out the contract:

1. Add a public phase after the whitelist phase where accounts not on the whitelist are allowed to mint
2. Add a `TokenMintingEvent` that we emit whenever a user calls the `mint` function successfully
3. Write Move unit tests to more efficiently test our code

### Adding a public phase

The simplest way to set a public phase is to add a start timestamp for both the public and whitelist minters. 

```rust
struct MintConfiguration has key {
    // ...
    start_timestamp_public: u64,
    start_timestamp_whitelist: u64,
}

const U64_MAX: u64 = 18446744073709551615;

public entry fun initialize_collection( ... ) {
    // ...
    move_to(&resource_signer, MintConfiguration {
        // ...
        // default to an impossibly distant future time to force owner to set this
        start_timestamp_whitelist: U64_MAX,
        start_timestamp_public: U64_MAX,
        // ...
    });
}
```

Then we enforce those restrictions in the mint function again.

We add an abort for trying to mint before the whitelist time, then check to see if the user is even on the whitelist. If they aren't, we abort if the public time hasn't come yet.

If the user is whitelisted and the whitelist time has begun or the public minting has begun, we finish our checks for `expiration_timestamp` and `minting_enabled`.

```rust
public entry fun mint(receiver: &signer, resource_addr: address) acquires MintConfiguration {
    // ...

    assert!(timestamp::now_seconds() >= mint_configuration.start_timestamp_whitelist, EWHITELIST_MINT_NOT_STARTED);
    // we are at least past the whitelist start. Now check for if the user is in the whitelist
    if (!table::contains(&mint_configuration.whitelist, signer::address_of(receiver))) {
        // user address is not in the whitelist, assert public minting has begun
        assert!(timestamp::now_seconds() >= mint_configuration.start_timestamp_public, EPUBLIC_MINT_NOT_STARTED);
    };

    // abort if this function is called after the expiration_timestamp
    assert!(timestamp::now_seconds() < mint_configuration.expiration_timestamp, error::permission_denied(ECOLLECTION_EXPIRED));
    // abort if minting is disabled
    assert!(mint_configuration.minting_enabled, error::permission_denied(EMINTING_DISABLED));

    // ...
}
```

Note that we haven't had a start time- we've been using the `minting_enabled` variable to gate access, but it's better design to have `minting_enabled` as a hard on/off switch for the contract and an actual start time for public and whitelist mints.

Our setter functions are nearly identical to `set_expiration_timestamp` just with a few additional checks to ensure our times make sense with each other:

```rust
public entry fun set_start_timestamp_public(
    admin: &signer,
    start_timestamp_public: u64,
    resource_addr: address,
) acquires MintConfiguration {
    // ...
    assert!(mint_configuration.start_timestamp_whitelist <= start_timestamp_public, EPUBLIC_NOT_AFTER_WHITELIST);
    // ...
}
public entry fun set_start_timestamp_whitelist(
    admin: &signer,
    start_timestamp_whitelist: u64,
    resource_addr: address,
) acquires MintConfiguration {
    // ...
    assert!(mint_configuration.start_timestamp_public >= start_timestamp_whitelist, EPUBLIC_NOT_AFTER_WHITELIST);
    // ...
}
```

### Adding custom events

In order to use events, we need to create a data structure that will be used to fill out the event data when it's emitted.

```rust
struct TokenMintingEvent has drop, store {
    token_receiver_address: address,
    creator: address,
    collection_name: String,
    token_name: String,
}
```

```rust
We need to create an EventHandle so we have somewhere to emit the events from: 
struct MintConfiguration has key {
    // ...
    token_minting_events: EventHandle<TokenMintingEvent>,
}
```

:::warning
Emitting events to the same resource is a bottleneck in this contract for parallelization. Check out our tutorials on how to parallelize contracts to remove this bottleneck.
:::

Initialize the `EventHandle` in the `initialize_collection` function and add the event emission function in `mint`:

```rust
public entry fun initialize_collection(...) {
    // ...

    move_to(&resource_signer, MintConfiguration {
        // ...
        token_minting_events: account::new_event_handle<TokenMintingEvent>(&resource_signer);
    });
}

public entry fun mint(receiver: &signer, resource_addr: address) acquires MintConfiguration {
    // ...

    event::emit_event<TokenMintingEvent>(
        &mut mint_configuration.token_minting_events,
        TokenMintingEvent {
            token_receiver_address: receiver_addr,
            creator: resource_addr,
            collection_name: mint_configuration.collection_name,
            token_name: mint_configuration.token_name,
        }
    );
}
```

Now whenever a user mints, a `TokenMintingEvent` will be emitted. You can view the events in a transaction on the Aptos explorer by looking up the transaction and viewing the Events section. Here are the events of the first transaction ever as an example: https://explorer.aptoslabs.com/txn/1/events?network=mainnet

Read more about events [here](https://aptos.dev/concepts/events/).

### Adding unit tests

So far, we've been making sure our code works by running it and checking if we get error codes as expected. This is a messy and inconsistent way of testing our code. It relies upon us not making any mistakes when running the commands in a specific order and that we run these checks every time we add new functionality.

We can leverage Move's native unit testing to create basic checks for our code that ensure our contract is working as expected. Read more about unit testing in Move [here](https://aptos.dev/move/move-on-aptos/cli/#compiling-and-unit-testing-move).

We'll make a simple list of every condition we've added to the contract, implicit or explicit, and ensure that when these conditions are met things go as expected and when they are not met, we get the error we expect.

Let's start with expected errors and when we'd expect to see them. We'll run a unit test for each of these error codes:

```rust
/// Action not authorized because the signer is not the admin of this module
const ENOT_AUTHORIZED: u64 = 1;
/// The collection minting is expired
const ECOLLECTION_EXPIRED: u64 = 2;
/// The collection minting is disabled
const EMINTING_DISABLED: u64 = 3;
/// The requested admin account does not exist
const ENOT_FOUND: u64 = 4;
/// The user account is not in the whitelist
const ENOT_IN_WHITELIST: u64 = 5;
/// Whitelist minting hasn't begun yet
const EWHITELIST_MINT_NOT_STARTED: u64 = 6;
/// Public minting hasn't begun yet
const EPUBLIC_MINT_NOT_STARTED: u64 = 7;
/// The public time must be after the whitelist time
const EPUBLIC_NOT_AFTER_WHITELIST: u64 = 8;
```

We also need to test that on-chain resources are changed accordingly if everything goes as expected. We'll refer to these as our positive testing conditions.

# Positive Test Conditions

1. When the collection is initialized, all on-chain resources are initialized in the resource account.
2. When the admin is changed, the next admin can successfully call admin-only functions.
3. When any functions that mutate resources are called, the resource on-chain is updated accordingly.
4. When a user mints successfully, they actually receive the NFT.

:::info
Running a basic test where everything goes right is called `happy path testing` in testing terminology. It's the most basic way of ensuring that running a program with no errors runs exactly as intended.
:::

When you're running a unit test with the Aptos Move CLI, the testing environment creates a sort of microcosm where your machine is initializing the entire blockchain and running it for a few seconds in order to simulate your unit tests.

This means that there are no accounts initialized anywhere, the time on-chain hasn't been set, and that you need to set all these things up when you begin your tests. We'll write a helper function that we call in each of our unit tests that initializes our testing environment.

Note that when you see `#[test_only]` above a function, it means the function is a function that can only be called in the test environment. `#[test]` marks a function as a unit test.

```rust
// dependencies only used in test, if we link without #[test_only], the compiler will warn us
#[test_only]
use aptos_std::token_objects::collection::{Self, Collection};
#[test_only]
use aptos_std::token_objects::aptos_token::{Self};
// ...etc


#[test_only]
fun setup_test(
    owner: &signer,
    new_admin: &signer,
    nft_receiver: &signer,
    nft_receiver2: &signer,
    aptos_framework: &signer,
    timestamp: u64,
) acquires MintConfiguration {
    timestamp::set_time_has_started_for_testing(aptos_framework);
    timestamp::update_global_time_for_test_secs(timestamp);
    account::create_account_for_test(signer::address_of(owner));
    account::create_account_for_test(signer::address_of(nft_receiver));
    account::create_account_for_test(signer::address_of(nft_receiver2));
    account::create_account_for_test(signer::address_of(aptos_framework));
    account::create_account_for_test(signer::address_of(new_admin));
    initialize_collection(
        owner,
        get_collection_name(),
        get_collection_uri(),
        MAXIMUM_SUPPLY,
        ROYALTY_NUMERATOR,
        ROYALTY_DENOMINATOR,
        get_token_name(),
        get_token_uri(),
    );
}

// Helper functions for the default values we've been using.
// We use these to avoid `utf8` casts, since we can't set `String` type const variables.
#[test_only]
const COLLECTION_NAME: vector<u8> = b"Krazy Kangaroos";
#[test_only]
public fun get_collection_name(): String { string::utf8(COLLECTION_NAME) }
// ...etc
```
We initialize the time on-chain, set it to `timestamp`, and then create accounts for all of our test accounts. Then we initialize the collection, since it's used in all of our test functions.

Now let's write our happy path. This tests that all the expected functionality is working as intended in a scenario where nothing goes wrong.

We'll write checks for our list #1-#4 above at the end of the test.

In a `#[test]` function, we can specify accounts we want to name, set their address, and pass them in as signers to the function as if they had signed the transaction. For all of our tests, we're going to use the same addresses for simplicity's sake.

Now let's pass them in as signers and set up our happy path test:

```rust

```

For the sake of brevity, we'll only explain a single example of a negative test condition here. We'll test that setting a new admin results in the old admin being unable to call admin-only functions:

```rust

```

:::tip
Calling the `error` module to emit a specific error function is useful in that it will print out the triple slash comment above the error code when you define it in your module. The error code can be derived by adding the error code value in `error.move` to the `const` value you set it to in your module.

That is, since we call `error::permission_denied(ENOT_AUTHORIZED)` we can derive the error code by knowing that `PERMISSION_DENIED` in `error.move` is `0x5`, and our `ENOT_AUTHORIZED` is `0x1`, so the error code will be `0x50001`.
:::


```shell
aptos move run --function-id default::create_nft_with_public_phase_and_events::set_expiration_timestamp \
               --profile default                           \
               --args                                      \
                   u64:YOUR_TIMESTAMP_IN_SECONDS_HERE      \
                   address:YOUR_RESOURCE_ADDRESS_HERE   
```

```shell
aptos move publish --named-addresses mint_nft_v2_part1=default --profile default --assume-yes
```