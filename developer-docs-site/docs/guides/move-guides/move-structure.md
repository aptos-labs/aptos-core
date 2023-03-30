---
title: "Aptos Move Structure"
slug: "move-structure"
---

# Aptos Move Structure

Take a moment to understand how Aptos suggest structuring your Move code.

Structs in Move resemble structs in other programming languages such as Rust, acting as classes of functions. You can have as many fields in a struct as you desire, but you cannot have a method on a struct, as in object-oriented programming. Similarly, there is no inheritance in Move. Instead, you would need to duplicate a struct to recreate it.

Once published, the definition of a struct in Move is immutable. Structs themselves are not upgradeable, although the values of their fields may change. For security in Move, only the module a struct is defined in may deconstruct the struct or access its properties.

## Abilities

[Structures](https://move-language.github.io/move/structs-and-resources.html) in Move can be given different [abilities](https://move-language.github.io/move/abilities.html) that describe what can be done with that type. There are four different abilities that allow:

* copy: values of types with this ability to be copied. A geographic ID would be a good use case. NFTs should not have this ability.
* drop: values of types with this ability to be popped/dropped.
* store: values of types with this ability to be saved or stored inside a struct in global storage.
* key: the type to serve as a key for global storage operations. With this ability, a value can be stored as a top-level item inside an account.

## Global storage

In Move, each account may have only one resource of a given type. This is because an account in Move resembles a hashmap whereby there will be only one `Coin` type, for instance. The hashmap is a mapping of resource type or module name to resource value. This is why Aptos offers the holder patterns of `CoinStore` and `TokenStore`, to provide an abstraction for holding multiple coins and tokens. These holders will contain tables or use generics for storage.

Aptos employs [Merkle trees](../../reference/glossary.md#merkle-trees) for efficient state synchronization and authenticated storage reads.

## Signers

In Aptos, signers are incredibly powerful. Structs are published under the signer address. Signers are generated when you sign and submit a transaction. When submitting the transaction, the signer is the first parameter by default. The signer has given consent to have their struct on chain. Signer does not have the Store or Key abilities, only the copy ability.

## key

To make signers available to other users, signers are stored in resources. The key ability allows the type to serve as a key for global storage operations, such as Coin having the Store ability. Since Balance has the key ability, you can store it as a top-level item inside an account.

Aptos does not store the signer but rather the signer capability. Only restricted native functions can create the signer capability. Minting an NFT requires access to the signer who created the collection. This is why many pre-mint the NFTs when conducting dynamic minting. Aptos provides resource accounts to sign transactions autonomously.

## acquires

Anytime you need to use any global resources, such as a struct, you should acquire it first. For example, both depositing and withdrawing an NFT acquire `TokenStore`. If you have a function in a different module that calls a function inside the module that acquires the resource, you don’t have to label the first function as `acquires()`.

This makes ownership clear since a resource is stored inside of an account. An account can decide if a resource may be created there. The module that defines that resource has power over reading and modifying that struct. So code inside that module needs to explicitly acquire that struct.

Still, anywhere you borrow or move in Move, you are automatically acquiring the resource. Use acquire for explicit inclusion for clarity. Similarly, the `exists()` function does not require the `acquires()` function.

Note: You can borrow global within your module from any account, from structs defined in your own module. You cannot borrow global outside of the module.

## move_to

You may then use the `move_to` function along with a reference to signer and account to move the struct into an account. In the process, we create a new instance of coin with value.


## Initialization

The `init_module` automatically gets called and run when the module is published:

```shell
    fun init_module(resource_account: &signer) {
        let resource_signer_cap = resource_account::retrieve_resource_account_cap(resource_account, @source_addr);
        let resource_signer = account::create_signer_with_capability(&resource_signer_cap);
```

The `mint_nft_ticket()` function gets a collection and creates a token.

With the resulting TokenData ID, the function uses the resource signer of the module to mint the token to an NFT receiver.

For example:
```shell
    public entry fun mint_nft(receiver: &signer) acquires ModuleData {
        let receiver_addr = signer::address_of(receiver);
```

## Signing

Any `entry fun` will take as the first parameter the type `&signer`. In both Move and Aptos, whenever you submit a transaction, the private key you sign the transaction with automatically makes the associated account the first parameter of the signer.

You can go from the signer to an address but normally not the reverse. So when claiming an NFT, both the private keys of the minter and receiver are needed, as shown in the instructions below.

In the `init_module`, the signer is always the account uploading the contract. This gets combined with:

```shell
        token::create_collection(&resource_signer, collection, description, collection_uri, maximum_supply, mutate_setting);

```
Then:

```shell
        signer_cap: account::SignerCapability,
```

The signer capability allows the module to sign autonomously. The [resource account](../resource-accounts.md) prevents anyone from getting the private key and is entirely controlled by the contract.

## Module data

The `ModuleData` is then initialized and *moved* to the resource account, which has the signer capability:

```shell
        move_to(resource_account, ModuleData {
```

In the `mint_nft_ticket()` function, the first step is borrowing the `ModuleData` struct:

```shell
        let module_data = borrow_global_mut<ModuleData>(@mint_nft);
```

And then use the reference to the signer capability in the  `ModuleData` struct to create the `resource_signer`:

```shell
        let resource_signer = account::create_signer_with_capability(&module_data.signer_cap);
```

In this manner, you can later use the signer capability already stored in module. When you move a module and its structs into an account, they become visible in [Aptos Explorer](https://explorer.aptoslabs.com/) associated with the account.

## Accounts

When you are minting an NFT, for example, the NFT is stored under your [account](../../concepts/accounts.md) address. When you submit a transaction, you sign the transaction. Find your account configuration information in `.aptos/config.yaml` relative to where you run `aptos init` (below).

[Resource accounts](../resource-accounts.md) allow the delegation of signing transactions. You create a resource account to grant a signer capability that can be stored in a new resource on the same account and can sign transactions autonomously. The signer capability is protected as no one has access to the private key for the resource account.
