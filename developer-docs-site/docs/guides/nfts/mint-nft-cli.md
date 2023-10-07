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

* the [Aptos CLI](../../tools/aptos-cli/install-cli/)
* the `aptos-core` repository cloned to your local machine: `git clone https://github.com/aptos-labs/aptos-core.git`
* a basic understanding of Move, NFTs and NFT Collections
* ideally, you've installed `jq`, a CLI tool to parse JSON values

## 1. Setup your CLI profile

To make things simple, let's initialize a profile and export its account address and the corresponding contract address to our environment variables, so we can easily re-use them later.

```shell title="Create the profiles"
echo '' | aptos init --profile mint_deployer --network devnet --assume-yes
echo '' | aptos init --profile nft_minter --network devnet --assume-yes
```

```shell title="Export them as shell environment variables"
export MINT_DEPLOYER=0x(aptos account lookup-address --profile mint_deployer | jq -r ".Result")
export NFT_MINTER=0x(aptos account lookup-address --profile nft_minter | jq -r ".Result")

echo "Mint deployer address    => $MINT_DEPLOYER"
echo "NFT minter address       => $NFT_MINTER"
```

If you didn't install `jq`, replace `jq -r ".Result"` with `grep "Result" | cut -d'"' -f4`.

## 1. Creating a simple smart contract to mint an NFT

We're going to start by making the simplest form of the flow for creating a collection and minting a token and sending it to a user. The code for this part of the tutorial is in the first section of the `aptos-core/aptos-move/move-examples/no_code_mint/1-Create-NFT` folder in your cloned `aptos-core` repository.

Here are the things we need to do first:

* Create the NFT collection and store the configuration options for it
* Mint a non-fungible token within that collection using the configuration options
* Send the minted token to a user account

### Defining the configuration options

The first thing we need to do is store the fields necessary to identify the collection and mint a token from it. We store these so we don't have to pass them in as fields to our mint function later.

```rust
// This struct stores all the relevant NFT collection and token's metadata
struct MintConfig has key {
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

We give `MintConfig` the `key` ability so that the module contract can store it as a global resource at an address. This means we can retrieve it in the contract later if we know of an address with the `MintConfig` resource.

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

```rust title="Create the MintConfig resource and move it to the deployer account which is also @no_code_mint_p1"
let mint_config = MintConfig {
    collection_name,
    creator: deployer_address,
    token_description: string::utf8(b""),
    token_name,
    token_uri,
    property_keys: vector<String>[string::utf8(b"given_to")],
    property_types: vector<String>[ string::utf8(b"address") ],
    property_values: vector<vector<u8>>[bcs::to_bytes(&deployer_address)],
};

// Move the MintConfig resource to the contract address itself, since deployer == @no_code_mint_p2
move_to(deployer, mint_config);
```

### Writing a simple mint function

First off, note the first few lines of the `mint_to` function. We assert that the caller here is the same as the account that owns the module:

```rust title="Ensure only the deployer can call this function"
public entry fun mint_to(
    deployer: &signer,
    receiver_address: address
) acquires MintConfig {
    let deployer_address = signer::address_of(deployer);
    assert!(deployer_address == @no_code_mint_p1, error::permission_denied(ENOT_AUTHORIZED));
    // ...
}
```

:::note
We define `@no_code_mint_p1` when we deploy the module with the `--named-addresses no_code_mint_p1=$MINT_DEPLOYER` flag.
:::

We then borrow the `MintConfig` resource from where we stored it in `init_module` and create the token with it:

```rust title="Populate the mint_token_object function with the data from our MintConfig resource and transfer it to the designated receiver"
// borrow the MintConfig resource and store it locally as mint_config
let mint_config = borrow_global_mut<MintConfig>(@no_code_mint_p1);

// mint the token object
let token_object = aptos_token::mint_token_object(
    deployer,
    mint_config.collection_name,
    mint_config.token_description,
    mint_config.token_name,
    mint_config.token_uri,
    mint_config.property_keys,
    mint_config.property_types,
    mint_config.property_values,
);

// transfer it to the receiver
object::transfer(deployer, token_object, receiver_address);
```

```rust title="Update the PropertyMap metadata on the token"
// Remember that prior to this, the `color` property on the token was a string "BLUE",
// taken from our original `MintConfig` resource:
property_keys: vector<String>[string::utf8(b"color")],
property_types: vector<String>[ string::utf8(b"string") ],
property_values: vector<vector<u8>>[bcs::to_bytes(&string::utf8(b"BLUE"))],

// we then update it to "RED" with:
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

Navigate to the `aptos-core/aptos-move/move-examples/no_code_mint/1-Create-NFT` folder and let's publish our module:

```rust title="Publish the module"
aptos move publish --named-addresses no_code_mint_p1=$MINT_DEPLOYER \
                   --profile mint_deployer \
                   --assume-yes
```

:::tip Move.toml configuration
The `Move.toml` file in the top directory for a Move contract should have a `Move.toml` file. This file specifies logistical things like the package name, dependencies and named addresses.

Here is where we declare that we want to specify our `no_code_mint_p1` named address in the contract when we publish the contract with the `--named-addresses no_code_mint_p1=$MINT_DEPLOYER` flag. This lets us use `@no_code_mint_p1` in our *.move files as an address.
:::

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

Note that `$MINT_DEPLOYER` can be used interchangeably with `mint_deployer` in the CLI, because `mint_deployer` is our named Aptos CLI profile.

```shell title="Construct the function call with the Aptos CLI"
aptos move run --function-id $MINT_DEPLOYER::create_nft::mint_to     \
               --profile mint_deployer                               \
               --args address:$NFT_MINTER                            \
               --assume-yes
```

After running that command, you should get back a JSON response with a bunch of fields. Take the transaction hash from those fields
and paste it into the URL below to see more details about the transaction:

https://explorer.aptoslabs.com/txn/YOUR_TRANSACTION_HASH_HERE/events?network=devnet

You should see a `0x4::collection::MintEvent` and a `0x1::object::TransferEvent`.

Congratulations! You've created a collection, minted an NFT, and transferred the NFT to another account.

:::tip Why don't I need to provide a `&signer` argument?
When you sign and submit a transaction with an account's private key, you automatically pass the first `&signer` parameter to the function.

Running an entry function with `--profile mint_deployer` signs and submits the transaction with your `mint_deployer` profile's account, which is why you don't need to provide the signer to the `--args` parameter list.
:::

## 2. Automating the mint function with a resource account

The issue with the code we've written so far is that it requires explicit approval from the creator to mint a token. The process isn't automated and the receiver doesn't ever approve of receiving the token.

The first step to improving the flow of this process is automating the creator's approval. We can create an Aptos framework Object that
manages the collection, where the Object itself is managed by the contract.

To achieve this, in this section we'll show you how to:

- Use a token base name that increments with the current collection supply, i.e., Token #1, Token #2, etc
- Create the NFT collection with an Aptos Object
- Store the `ExtendRef` for the Object, which gives us the ability to produce a `&signer` value for it
- Automate minting the token to the user; that is, write a mint function that works without the collection creator's signature

First, let's go over how to increment the token's name with a number in it that matches the current collection supply:

```rust title="Converting a number to a string and appending it to another string"
    inline fun u64_to_string(value: u64): String {
        // ... some clever logic to convert a u64 to a utf8 character
    }

    inline fun concat_u64(s: String, n: u64): String {
        let n_str = u64_to_string(n);
        string::append(&mut s, n_str);
        s
    }

    inline fun get_collection_supply(creator_addr: address): u64 {
      option::extract(&mut collection::count(object::address_to_object<Collection>(creator_addr)))
    }
```

```rust title="When we mint the token, we construct the token name from the base name + the supply"
public entry fun mint(receiver: &signer) acquires MintConfig {
  // ...
  // Note that we changed the `token_name` field in `MintConfig` to `token_base_name`.
  // We can now call `concat_u64(string::utf8(b"Token #"), 0)` to get a string: `Token #0`
  // Append the collection supply to the base name to create the token name
  let obj_creator_addr = object::address_from_extend_ref(&extend_ref);
  let collection_supply = get_collection_supply(obj_creator_addr);
  let full_token_name = concat_u64(mint_config.token_base_name, collection_supply);

  let token_object = aptos_token::mint_token_object(
    // ...
    full_token_name,
    // ...
  );

  // ...
}
```

### What is an Aptos Object?

In short, an Aptos Object is a core Aptos primitive that facilitates resource management and complex, on-chain representations of data at a single address. If you'd like to read more, head over to the [Object](../../standards/aptos-object) page.

If you want to manage an account's resources but don't want to be present to approve of it, you can write Move code to automate it by creating an Object, storing its [ExtendRef](https://aptos.dev/standards/aptos-object/#object-capabilities-refs) in a resource which you can use programmatically to generate the Object's `&signer`.
### Making an Object our collection creator

Most of the code for our contract in this second part is very similar, so we're only going to discuss the parts added that make it different.

First off, we need to store the Object's `ExtendRef` somewhere when we create it. Add it to the `MintConfig` resource:

```rust
#[resource_group_member(group = aptos_framework::object::ObjectGroup)]
struct MintConfig has key {
  extend_ref: ExtendRef, // this is how we generate the Object's `&signer`
  collection_name: String,
  token_description: String,
  token_name: String,
  token_uri: String,
  property_keys: vector<String>,
  property_types: vector<String>,
  property_values: vector<vector<u8>>,
}
```

:::tip Advanced Tip
Note the new `resource_group_member` tag. Any resources with this tag marks it as part of an Object's ObjectGroup resource group. This groups together the individual resources on an Object at the storage layer, (generally) resulting in faster, more efficient data storage and retrieval.

See [object resource groups](https://aptos.dev/standards/aptos-object/#object-resource-group).
:::

Now let's examine how we create the object and store its ExtendRef in the MintConfig resource for later.

```rust title="Create an object and have it create the collection. Store its ExtendRef in MintConfig"
// Create an object that will be the collection creator
// The object will be owned by the deployer account
let constructor_ref = object::create_object(deployer);
// generate its &signer to create the collection
let obj_signer = object::generate_signer(&constructor_ref);

aptos_token::create_collection(
    obj_signer, // the object is now the creator of the collection
    // ...
);

// generate the ExtendRef with the returned ConstructorRef from the creation function
let extend_ref = object::generate_extend_ref(&constructor_ref);

// store it in the MintConfig resource
let mint_config = MintConfig {
    extend_ref,
    // ...
};

// Move the MintConfig resource to the contract address itself, since deployer == @no_code_mint_p2
move_to(deployer, mint_config);
```

Now our object, owned by `deployer`, will actually be the collection creator!

We now need to replace the `mint_to` function with a `mint` function, where instead of having the creator send a token to a designated receiver, we send the token directly to the `receiver`, only when they request it.

```rust title="The receiver: &signer argument means the receiver requests to mint now, instead of the creator."
public entry fun mint(receiver: &signer) acquires MintConfig {
  // get our contract data at the module address
  let mint_config = borrow_global_mut<MintConfig>(@no_code_mint_p2);

  // borrow the object's ExtendRef and use it to generate the object's &signer
  let extend_ref = &mint_config.extend_ref;
  let obj_signer = object::generate_signer_for_extending(&mint_config.extend_ref);
  // ...
  // the rest of the function uses `obj_signer` like we used `deployer` (aka: you) before
  // ... transfer the newly minted Token Object to `receiver`
}
```

Now the `receiver` only has to request to mint an NFT, and the object creator will mint it and send it over.
You, the deployer, no longer need to be involved in the transaction!

This means the contract is fully automated now- however, tihs function has no form of access control, meaning anyone can mint a token as many times as they want!

Next section, we'll examine how to implement gating access to the `mint` function.

But for now, let's publish the module and run the contract.

### Publishing the module and running the contract

Publishing the module is basically the same as before. Just make sure you're in the `2-Object-as-Creator` directory and run this command:

```shell title="Publish the new module. The only difference here is the no_code_mint_p2 address"
aptos move publish --named-addresses no_code_mint_p2=$MINT_DEPLOYER \
                   --profile mint_deployer                          \
                   --assume-yes
```

```shell title="Mint the token, except this time with the nft_minter profile"
aptos move run --function-id $MINT_DEPLOYER::object_as_creator::mint     \
               --profile nft_minter                                      \
               --assume-yes
```

Great! Now you've created the collection as an owner and requested to mint as a user and received the newly minted NFT.

It may not feel different since you're acting as the owner and the receiver all from the command line, but in an actual dapp this user flow makes much more sense than before.

In the first section, the user has to wait for the owner of the contract to mint and send them an NFT. In the second section, the user can request to mint and receive an NFT themselves.

Look up the transaction hash on the [Aptos explorer](https://explorer.aptoslabs.com/?network=devnet) if you'd like- although for the most part, the transaction appears very similarly to before.

## 3. Adding restrictions with an allowlist

We're still missing some very common features for NFT minting contracts:

1. An allowlist that restricts minting to allowlisted addresses
2. The ability to add and remove addresses from the allowlist
3. A start and end time
4. The ability to enable or disable the mint
5. Setting a price to mint

We can add these by creating an allowlist with `allowlist.move` and then simply gating our `mint` function with the allowlist's `try_increment` function.

Explaining the inner workings of the `allowlist.move` and how to write it is beyond the scope of this tutorial, but let's at least review how to use it:

### How the allowlist works

For each tier, you **must** specify the following:
 - Tier name
 - Open to the public, that is, is it an allowlist (not public) or merely a registry that tracks # of mints (public)
 - Price
 - Start time
 - End time
 - Per user limit (# of mints)

If a user exists under multiple allowlists, the allowlist contract will mint from the earliest, cheapest one.

### Configuring the allowlist

For the most part, `allowlist.move` handles all the configuration options for us. We merely have to create the allowlist with the object creator and then call the `try_increment(...)` function later.

In our `init_module` function we initialize a `public` tier and a `private` tier. We customize it so that anyone can mint once for free, but only our named address `@allowlisted_minter` can mint an additional 10 times for free.

```rust title="Initialize the allowlist and add a vector of addresses to the newly created public tier"
// Add a public allowlist tier that lets anyone mint 1 time for free
allowlist::upsert_tier_config(
    &obj_signer,
    string::utf8(b"public"),
    true, // open_to_public,
    0, // price in APT
    timestamp::now_seconds(), // start_time
    timestamp::now_seconds() + 1000000000, // end_time
    1, // per_user_limit
);
// Note that we don't need to call `add_to_tier` for the `public` tier, since the tier is open to the public.

// Add a private allowlist tier that lets only specific addresses mint 10 times for free
allowlist::upsert_tier_config(
    &obj_signer,
    string::utf8(b"private"),
    false, // open_to_public,
    0, // price in APT
    timestamp::now_seconds(), // start_time
    timestamp::now_seconds() + 1000000000, // end_time
    10, // per_user_limit
);

// Add our allowlisted address to it
allowlist::add_to_tier(&obj_signer, tier_name, vector<address> [@allowlisted_minter]);
```

Since the allowlist is managed by the `allowlist.move` contract with interfacing functions, we merely plug it into our `mint` function with `try_increment`:

```rust title="Gate access to the minting function by calling try_increment(...)"
// This forces the receiver to pay the mint price and ensures they're in a valid allowlist
// tier with at least one mint left.
// @allowlisted_minter will get 11 free mints (10 from the private tier, 1 from the public tier)
// everyone else will get 1 (from the public tier)
public entry fun mint(...) {
    allowlist::try_increment(
        &obj_signer,
        receiver,
    );
}
```

And other than that, the contract is largely the exact same as before! 

### Publishing the module and running the contract

Navigate to the `3-Mint-with-Allowlist` directory and publish the module for part 3:

```shell title="Publish the new module. We now have to specify the named address allowlisted_minter"
aptos move publish --named-addresses no_code_mint_p3=$MINT_DEPLOYER,allowlisted_minter=nft_minter \
                   --profile mint_deployer                                                        \
                   --assume-yes
```

Now let's mint as both `mint_deployer` and `nft_minter`. The # of successful mints should be 1 and 3 respectively.

```shell title="Mint a token as 'mint_deployer' - this only succeeds one time."
aptos move run --function-id $MINT_DEPLOYER::mint_with_allowlist::mint   \
               --profile mint_deployer                                   \
               --assume-yes
```

If you run the above function twice, you'll get the following error code:

`Move abort in 0x...::allowlist: EACCOUNT_NOT_ELIGIBLE(0x50001): The account requesting to mint is not eligible to do so.`

This is because the deployer isn't allowlisted! It's only permitted 1 free mint.

Then, try minting from `nft_minter` multiple times. It should allow you 3 mints before giving you an error!

```shell title="Mint a token as 'nft_minter' - this succeeds 3 times before failing."
aptos move run --function-id $MINT_DEPLOYER::mint_with_allowlist::mint   \
               --profile nft_minter                                      \
               --assume-yes
```

## 4. Adding token metadata

