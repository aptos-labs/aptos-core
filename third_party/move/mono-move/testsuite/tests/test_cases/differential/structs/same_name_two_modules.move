// Types with the same name in two modules at one address have distinct
// layouts and distinct resource keys.

// RUN: publish
module 0x42::a {
    struct P has copy, drop { x: u8, y: u64 }
    struct R has key, drop { v: u8 }
    public fun mk(small: u8, medium: u64): P { P { x: small, y: medium } }
    public fun sum(value: &P): u64 { (value.x as u64) + value.y }
    public fun put(account: &signer, value: u8) { move_to(account, R { v: value }) }
    public fun get(addr: address): u64 { if (exists<R>(addr)) (borrow_global<R>(addr).v as u64) else 0 }
}
module 0x42::b {
    struct P has copy, drop { y: u64, x: u8, z: u128 }
    struct R has key, drop { v: u128, w: vector<u8> }
    public fun mk(small: u8, medium: u64, large: u128): P { P { y: medium, x: small, z: large } }
    public fun sum(value: &P): u64 { (value.x as u64) + value.y + (value.z as u64) }
    public fun put(account: &signer, value: u128) { move_to(account, R { v: value, w: b"bb" }) }
    public fun get(addr: address): u64 { if (exists<R>(addr)) (borrow_global<R>(addr).v as u64) else 0 }
}
module 0x42::user {
    public fun both(): u64 {
        let pa = 0x42::a::mk(1, 20);
        let pb = 0x42::b::mk(3, 400, 5000);
        let qa = pa;
        0x0::test_utils::force_gc();
        0x42::a::sum(&qa) + 0x42::b::sum(&pb)
    }

    public fun resources(account: signer): u64 {
        let addr = std::signer::address_of(&account);
        0x42::a::put(&account, 7);
        let only_a = 0x42::b::get(addr);
        0x42::b::put(&account, 900);
        0x42::a::get(addr) * 1000 + 0x42::b::get(addr) + only_a
    }

    public fun put_both(account: signer) {
        0x42::a::put(&account, 1);
        0x42::b::put(&account, 2);
    }
}

// RUN: execute 0x42::user::both --heap-size 512
// CHECK: results: 5424

// RUN: execute 0x42::user::resources --args 0x42
// CHECK: results: 7900

// RUN: execute 0x42::user::put_both --args 0x43 --commit
// CHECK: writes: create 0x42::a::R at 0x43 0x01; create 0x42::b::R at 0x43 0x02000000000000000000000000000000026262
