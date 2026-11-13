// A closure over a private `entry` function cannot be stored: only `public`
// functions and functions marked `#[persistent]` are storable, and `entry`
// alone does not count.

//# publish
// Source: a private `entry` function, stored in a resource.
module 0x66::private_entry {
    struct Work(|u64|u64) has store, key;

    entry fun target(x: u64): u64 {
        x + 1
    }

    entry fun store(s: &signer) {
        move_to(s, Work(target))
    }
}

//# publish
// Source: a `public entry` function, stored, loaded and called.
module 0x66::public_entry {
    struct Work(|u64|u64) has store, key;

    public entry fun target(x: u64): u64 {
        x + 1
    }

    entry fun store(s: &signer) {
        move_to(s, Work(target))
    }

    entry fun exec(a: address): u64 acquires Work {
        let Work(f) = move_from<Work>(a);
        f(1)
    }
}

//# run 0x66::public_entry::store --signers 0x66

//# run 0x66::public_entry::exec --args @0x66

//# publish
// Source: a private `entry` function marked `#[persistent]`.
module 0x66::persistent_entry {
    struct Work(|u64|u64) has store, key;

    #[persistent]
    entry fun target(x: u64): u64 {
        x + 1
    }

    entry fun store(s: &signer) {
        move_to(s, Work(target))
    }

    entry fun exec(a: address): u64 acquires Work {
        let Work(f) = move_from<Work>(a);
        f(1)
    }
}

//# run 0x66::persistent_entry::store --signers 0x66

//# run 0x66::persistent_entry::exec --args @0x66
