// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A generic spec variable is a ghost resource per instantiation. An update
// leaves the rest of memory in the frame, in the function and in its callers.
module 0x42::generic_spec_vars {
    struct Coin<phantom T> has store { value: u64 }
    struct Registry has key { entries: u64 }
    struct A {}
    struct B {}

    spec module {
        global minted<T>: num;
    }

    public fun mint<T>(value: u64): Coin<T> {
        spec {
            update minted<T> = minted<T> + value;
        };
        Coin { value }
    }
    spec mint {
        aborts_if false;
        ensures minted<T> == old(minted<T>) + value;
        ensures result.value == value;
    }

    public fun burn<T>(coin: Coin<T>): u64 {
        let Coin { value } = coin;
        spec {
            update minted<T> = minted<T> - value;
        };
        value
    }

    // A generic caller's update goes through at its own type argument.
    public fun mint_twice<T>(value: u64): (Coin<T>, Coin<T>) {
        (mint<T>(value), mint<T>(value))
    }
    spec mint_twice {
        aborts_if false;
        ensures minted<T> == old(minted<T>) + 2 * value;
    }

    // Different instantiations are different memory.
    public fun mint_a(value: u64): Coin<A> {
        mint<A>(value)
    }
    spec mint_a {
        aborts_if false;
        ensures minted<A> == old(minted<A>) + value;
        ensures minted<B> == old(minted<B>);
    }

    // An update does not open the frame of a caller.
    public fun mint_keeps_registry(account: address, value: u64): Coin<A> {
        mint<A>(value)
    }
    spec mint_keeps_registry {
        aborts_if false;
        ensures global<Registry>(account) == old(global<Registry>(account));
    }

    public fun wrong_amount<T>(value: u64): Coin<T> {
        mint<T>(value)
    }
    spec wrong_amount {
        aborts_if false;
        ensures minted<T> == old(minted<T>) + value + 1; // error: the update adds `value`
    }

    // An `update` of the function's own specification holds at each return,
    // and callers see it through the contract.
    public fun count(value: u64): u64 {
        value
    }
    spec count {
        pragma opaque;
        aborts_if false;
        ensures result == value;
        update minted<A> = minted<A> + value;
    }

    public fun count_twice(value: u64): u64 {
        count(value);
        count(value)
    }
    spec count_twice {
        aborts_if false;
        ensures minted<A> == old(minted<A>) + 2 * value;
        ensures minted<B> == old(minted<B>);
    }

    // The same update in the body too does not hold against the specification's.
    public fun count_again(value: u64): u64 {
        spec {
            update minted<A> = minted<A> + value;
        };
        value
    }
    spec count_again { // error: updates twice
        aborts_if false;
        update minted<A> = minted<A> + value;
    }

    // An update reads a reference-typed value through the reference.
    public fun read_through(value: &mut u64): u64 {
        spec {
            update minted<B> = value;
        };
        *value
    }
    spec read_through {
        aborts_if false;
        ensures minted<B> == old(value);
    }

    // A parameter named `_` is an ordinary parameter of the rendering.
    public fun ignored(_: &Registry): u64 {
        0
    }
    spec ignored {
        aborts_if false;
        ensures result == 0;
    }

    // A resource group attribute names its group in a module.
    #[resource_group(scope = global)]
    struct Group {}

    #[resource_group_member(group = 0x42::generic_spec_vars::Group)]
    struct Member has key { value: u64 }
}
