// `exists` on a plain resource, a generic instantiation and a resource with a vector field,
// and as a branch condition; the golden pins how each lowers. Only G<vector<u64>> is
// stored, so checking the wrong instantiation changes the result of `instantiated`.

// RUN: publish --print(micro-ops)
module 0x42::ex {
    struct R has key { v: u64 }
    struct G<T> has key { t: T }
    struct V has key { data: vector<u64> }

    public fun plain(addr: address): bool { exists<R>(addr) }

    public fun put_vec(account: signer) { move_to(&account, G<vector<u64>> { t: vector[5] }) }

    public fun instantiated(addr: address): u8 {
        let bits = if (exists<G<u8>>(addr)) 1 else 0;
        if (exists<G<vector<u64>>>(addr)) bits = bits | 2;
        bits
    }

    public fun as_branch(account: signer, addr: address): u64 {
        if (!exists<V>(addr)) move_to(&account, V { data: vector[1, 2] });
        if (exists<V>(addr)) 1 else 0
    }
}

// RUN: execute 0x42::ex::plain --args 0x42
// CHECK: results: false

// RUN: execute 0x42::ex::put_vec --args 0x42 --commit
// CHECK: writes: create 0x42::ex::G<vector<u64>> at 0x42 0x010500000000000000

// RUN: execute 0x42::ex::instantiated --args 0x42
// CHECK: results: 2

// RUN: execute 0x42::ex::as_branch --args 0x42, 0x42
// CHECK: results: 1
