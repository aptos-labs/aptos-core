---
title: "Common questions"
id: "common-questions"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Common questions
## How are resource accounts created?

Let's review the two functions used to create resource accounts and what they return.

First off, note that both creation functions allow for the input of a [**seed**](./common-questions.md#whats-a-seed) byte vector for the [ensuing hash used to compute the resource address](#how-is-the-address-for-a-resource-account-derived).

<Tabs groupId="creation">
  <TabItem value="account.move" label="account.move">

The `account.move` version creates a resource account and rotates its authentication key to `0x0`. The resulting resource account doesn't have an associated private key and can only be controlled through the usage of the [SignerCapability](#whats-a-signercapability) returned by the creation function.

```rust title="Creating a resource account in account.move"
public fun create_resource_account(
    source: &signer,
    seed: vector<u8>
): (signer, SignerCapability) acquires Account {
    let resource_addr = create_resource_address(&signer::address_of(source), seed);

    // ...

    // By default, only the SignerCapability should have control over the resource account and not the auth key.
    // If the source account wants direct control via auth key, they would need to explicitly rotate the auth key
    // of the resource account using the SignerCapability.
    rotate_authentication_key_internal(&resource, ZERO_AUTH_KEY);

    let account = borrow_global_mut<Account>(resource_addr);
    account.signer_capability_offer.for = option::some(resource_addr);
    let signer_cap = SignerCapability { account: resource_addr };
    (resource, signer_cap)
}
```
  </TabItem>
  <TabItem value="resource_account.move" label="resource_account.move">

The `create_resource_account` function in `resource_account.move` below creates a resource account and rotates its authentication key to the `optional_auth_key` argument. If this field is an empty vector, the authentication key is rotated to the `origin` account's authentication key.

```rust title="Creating a manually controlled resource account in resource_account.move"
public entry fun create_resource_account(
    origin: &signer,
    seed: vector<u8>,
    optional_auth_key: vector<u8>,
) acquires Container {
    let (resource, signer_cap) = account::create_resource_account(origin, seed);
    rotate_account_authentication_key_and_store_capability(
        origin,
        resource,
        signer_cap,
        optional_auth_key,
    );
}
```
The resource account created from this is functionally very similar to a user account that has had its authentication key rotated (see: [Rotating an authentication key](../account-management/key-rotation.md)), because it cannot yet be controlled programmatically and can still be controlled by a private key.

However, there does exist a SignerCapability for the resource account, it just isn't being used yet. To enable programmatic control, you would need to [retrieve the SignerCapability.](./managing-resource-accounts#retrieving-a-signercapability)

The end result of a creating a resource account with `create_resource_account(...)` in `resource_account.move` and then retrieving the SignerCapability is the same as creating the resource account with `create_resource_account(...)` in `account.move`.
  </TabItem>
</Tabs>

## What's a seed?

A seed is an optional user-specified byte vector that is input during the creation of a resource account. The seed is used to ensure that the resulting resource account's address is unique.

Since the hash function used to derive the resource account's address is deterministic, providing the same seed and source address will always result in the same resource account address.

This also means that without a seed, if you were to try to generate multiple resource accounts from a single source account, you would end up with the same address each time due to a collision in the hashing computation.

Thus, the `seed: vector<u8>` argument facilitates the creation of multiple resource accounts from a single source account and also allows for deterministically deriving a resource account address given an address and a seed byte vector.

## What's a SignerCapability?

A SignerCapability is a simple but powerful resource that allows a developer to programmatically manage a resource account. This is achieved with the `create_signer_with_capability` function:

```rust
public fun create_signer_with_capability(capability: &SignerCapability): signer {
    let addr = &capability.account;
    create_signer(*addr)
}
```

The SignerCapability resource doesn't actually *do* anything special on its own, it's juts an abstract representation of permission to generate a [`signer`](../../move/book/signer.md) primitive for the account it was created for.

It contains a single field called `account`, which is just the address that it has permission to generate a `signer` for:

```rust
struct SignerCapability has drop, store {
    account: address
}
```

Since it only has the abilities `drop` and `store`, it can't be copied, meaning only `account.move` itself can manage the new creation of a `SignerCapability`. The inner `account` field cannot be altered post creation, so it can only sign for the resource account it was initially created for.


## What's stopping someone from using my SignerCapability?

You might be wondering "*Why does this work? Isn't it dangerous to be able to create a signer for an account so easily?*"

Move's [privileged struct operations](../../move/book/structs-and-resources#privileged-struct-operations) require that creating structs and accessing their inner fields can only occur from within the module that defines the struct. This means that unless the developer provides a public accessor function to a stored `SignerCapability`, there is no way for another account to gain access to it.

:::warning
Be mindful of properly gating access to a function that uses a `SignerCapability` and be extra careful when returning a `signer` from a public function, since this gives the caller unrestricted access to control the account.
:::

A good rule of thumb is to by default set all functions that return a `SignerCapability` or a `signer` to internal private functions unless you've very carefully thought about the implications.

```rust title="An example of a private function that returns a signer"
use aptos_std::resource_account;

// The function name `internal_get_signer()` is functionally no different than
// `get_signer()`, but it signifies to any developer that comes across it that
// it shouldn't be publically accessible without careful thought.
fun internal_get_signer(): signer {
    // Borrow the signer cap you've stored somewhere
    let signer_cap = borrow_global<SignerCap>(@your_contract).signer_cap;

    // Return the signer generated from it
    resource_account::create_signer_with_capability(&signer_cap)
}
```

## How is the address for a resource account derived?

When a resource account is created, the address is derived from a SHA3-256 hash of the requesting account's address, a byte scheme to identify it as a resource account, and an optional user-specified byte vector [**seed**](./common-questions.md#whats-a-seed). Here is the implementation of `create_resource_address` function in `account.move`:
```rust
/// This is a helper function to compute resource addresses. Computation of the address
/// involves the use of a cryptographic hash operation and should be use thoughtfully.
public fun create_resource_address(source: &address, seed: vector<u8>): address {
    let bytes = bcs::to_bytes(source);
    vector::append(&mut bytes, seed);
    vector::push_back(&mut bytes, DERIVE_RESOURCE_ACCOUNT_SCHEME);
    from_bcs::to_address(hash::sha3_256(bytes))
}
```

## Why am I getting the `EACCOUNT_ALREADY_EXISTS` error?

If you're getting this error while trying to create a resource account, it's because you have already created a resource account with that specific [**seed**](./common-questions.md#whats-a-seed).

This error occurs because there's a collision in the output of the hashing function used to derive a resource account's address.

To fix it, you need to change the seed or the function you're calling will continue to unsuccessfully attempt to create an account at an address that already exists.
