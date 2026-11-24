// RUN: publish
module 0x42::fv_store {
    struct Holder has key, drop {
        f: |u64|u64 has copy + drop + store,
    }

    #[persistent]
    fun add(a: u64, b: u64): u64 {
        a + b
    }

    #[persistent]
    fun tally(data: vector<u8>, extra: u64): u64 {
        data.length() + extra
    }

    public fun store(s: signer) {
        let data = vector[1u8, 2u8];
        move_to(&s, Holder { f: |k| tally(data, k) });
    }

    public fun call(a: address, x: u64): u64 acquires Holder {
        let f = borrow_global<Holder>(a).f;
        f(x)
    }

    public fun put_back(s: signer, a: address) acquires Holder {
        let holder = move_from<Holder>(a);
        move_to(&s, holder);
    }

    public fun call_and_put_back(s: signer, a: address, x: u64): u64 acquires Holder {
        let holder = move_from<Holder>(a);
        let f = holder.f;
        let result = f(x);
        move_to(&s, holder);
        result
    }

    public fun overwrite_with_equal(a: address) acquires Holder {
        let data = vector[1u8, 2u8];
        borrow_global_mut<Holder>(a).f = |k| tally(data, k);
    }

    public fun overwrite_with_different(a: address) acquires Holder {
        borrow_global_mut<Holder>(a).f = |b| add(7, b);
    }
}

// RUN: execute 0x42::fv_store::store --args 0x42 --commit
// CHECK: writes: create 0x42::fv_store::Holder at 0x42 0x07010000000000000000000000000000000000000000000000000000000000000000420866765f73746f72650574616c6c790001000000000000000501020102

// RUN: execute 0x42::fv_store::call --args 0x42, 10 --commit
// CHECK: results: 12 | writes: none

// Expected divergence: an uncalled stored closure moved out and back.
// RUN: execute 0x42::fv_store::put_back --args 0x42, 0x42 --commit
// CHECK-V1: writes: modify 0x42::fv_store::Holder at 0x42 0x07010000000000000000000000000000000000000000000000000000000000000000420866765f73746f72650574616c6c790001000000000000000501020102
// CHECK-V2: writes: none

// Expected divergence: moved out, called, and moved back.
// RUN: execute 0x42::fv_store::call_and_put_back --args 0x42, 0x42, 1 --commit
// CHECK-V1: results: 3 | writes: modify 0x42::fv_store::Holder at 0x42 0x07010000000000000000000000000000000000000000000000000000000000000000420866765f73746f72650574616c6c790001000000000000000501020102
// CHECK-V2: results: 3 | writes: none

// Expected divergence: an equal closure written over the stored one.
// RUN: execute 0x42::fv_store::overwrite_with_equal --args 0x42 --commit
// CHECK-V1: writes: modify 0x42::fv_store::Holder at 0x42 0x07010000000000000000000000000000000000000000000000000000000000000000420866765f73746f72650574616c6c790001000000000000000501020102
// CHECK-V2: writes: none

// Only MonoMove runs, so its bytes are committed and V1 reads them next.
// RUN: execute 0x42::fv_store::overwrite_with_different --args 0x42 --commit
// CHECK-V2: writes: modify 0x42::fv_store::Holder at 0x42 0x07010000000000000000000000000000000000000000000000000000000000000000420866765f73746f726503616464000100000000000000020700000000000000

// RUN: execute 0x42::fv_store::call --args 0x42, 1
// CHECK: results: 8
