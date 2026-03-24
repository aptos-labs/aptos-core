// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
// (no language-version flag needed: reads/writes in spec blocks are allowed at 2.4)

// Tests frame conditions derived from `modifies_of`/`reads_of` declarations on
// function parameters. These annotations tell the prover which resources
// a function parameter may read or write, enabling frame condition inference
// for the parameter variant in the apply procedure.

module 0x42::param_access {
    struct Data  has key { value: u64 }
    struct Index has key { pos: u64 }

    // =========================================================================
    // Library
    // =========================================================================

    /// Opaque read-only function accessing both Data and Index.
    fun read_indexed(addr: address): u64 acquires Data, Index {
        Data[addr].value + Index[addr].pos
    }
    spec read_indexed {
        pragma opaque;
        pragma aborts_if_is_partial;
        ensures result == Data[addr].value + Index[addr].pos;
    }

    // =========================================================================
    // Higher-order wrapper with reads_of (reads only)
    // =========================================================================

    fun apply_reads(f: |address| u64, x: address): u64 {
        f(x)
    }
    spec apply_reads {
        pragma opaque;
        pragma verify = false;
        reads_of<f> Data, Index;
        ensures result == result_of<f>(x);
        ensures ensures_of<f>(x, result);
    }

    // =========================================================================
    // 1. Reads reads_of — success
    // =========================================================================

    /// With reads_of declaring reads-only, both resources are unchanged.
    fun test_reads_of(addr: address): u64 acquires Data, Index {
        apply_reads(|a| read_indexed(a) spec {
            ensures result == Data[a].value + Index[a].pos;
        }, addr)
    }
    spec test_reads_of {
        pragma aborts_if_is_partial;
        ensures result == Data[addr].value + Index[addr].pos;
        ensures Data[addr] == old(Data[addr]);
        ensures Index[addr] == old(Index[addr]);
    }

    // =========================================================================
    // 2. Reads reads_of — failure
    // =========================================================================

    /// Wrong result claim.
    fun test_reads_of_wrong(addr: address): u64 acquires Data, Index {
        apply_reads(|a| read_indexed(a) spec {
            ensures result == Data[a].value + Index[a].pos;
        }, addr)
    }
    spec test_reads_of_wrong {
        pragma aborts_if_is_partial;
        ensures result == Data[addr].value * Index[addr].pos; // error: result is value + pos, not value * pos
    }

    // =========================================================================
    // 3. Writes modifies_of — parameter variant writes + returns post-state value
    // =========================================================================

    /// Opaque function that writes Data and returns the new value.
    fun set_data(addr: address, v: u64): u64 acquires Data {
        Data[addr].value = v;
        Data[addr].value
    }
    spec set_data {
        pragma opaque;
        modifies Data[addr];
        ensures result == v;
        ensures Data[addr].value == v;
        aborts_if !exists<Data>(addr);
    }

    fun apply_writes(f: |address| u64, x: address): u64 {
        f(x)
    }
    spec apply_writes {
        pragma opaque;
        pragma verify = false;
        modifies Data[x];
        modifies_of<f>(a: address) Data[a];
        ensures ensures_of<f>(x, result);
        aborts_if aborts_of<f>(x);
    }

    /// Positive: parameter variant with writes, result comes from post-state.
    fun test_writes_modifies_of(addr: address): u64 acquires Data {
        apply_writes(|a| set_data(a, 99) spec {
            modifies Data[a];
            ensures result == 99;
            ensures Data[a].value == 99;
            aborts_if !exists<Data>(a);
        }, addr)
    }
    spec test_writes_modifies_of {
        aborts_if !exists<Data>(addr);
        ensures result == 99;
    }

    /// Negative: wrong result claim for writes parameter variant.
    fun test_writes_modifies_of_wrong(addr: address): u64 acquires Data {
        apply_writes(|a| set_data(a, 99) spec {
            modifies Data[a];
            ensures result == 99;
            ensures Data[a].value == 99;
            aborts_if !exists<Data>(a);
        }, addr)
    }
    spec test_writes_modifies_of_wrong {
        aborts_if !exists<Data>(addr);
        ensures result == 0; // error: post-condition does not hold
    }

    // =========================================================================
    // 4. Mixed reads_of/modifies_of — Config is read-only, Data is writable
    // =========================================================================

    struct Config has key { active: bool }

    fun apply_mixed(f: |address| u64, x: address): u64 {
        f(x)
    }
    spec apply_mixed {
        pragma opaque;
        pragma verify = false;
        modifies Data[x];
        reads_of<f> Config;
        modifies_of<f>(a: address) Data[a];
        ensures ensures_of<f>(x, result);
        aborts_if aborts_of<f>(x);
    }

    /// Opaque function that writes Data conditionally on Config.
    fun conditional_set(addr: address): u64 acquires Data, Config {
        if (Config[addr].active) { Data[addr].value = 77 };
        Data[addr].value
    }
    spec conditional_set {
        pragma opaque;
        modifies Data[addr];
        ensures Config[addr].active ==> result == 77;
        ensures Config[addr].active ==> Data[addr].value == 77;
        aborts_if !exists<Data>(addr);
        aborts_if !exists<Config>(addr);
    }

    /// Positive: mixed access — Config unchanged (reads-only), result depends on Config.
    fun test_mixed_of(addr: address): u64 acquires Data, Config {
        apply_mixed(|a| conditional_set(a) spec {
            modifies Data[a];
            ensures Config[a].active ==> result == 77;
            ensures Config[a].active ==> Data[a].value == 77;
            aborts_if !exists<Data>(a);
            aborts_if !exists<Config>(a);
        }, addr)
    }
    spec test_mixed_of {
        aborts_if !exists<Data>(addr);
        aborts_if !exists<Config>(addr);
        ensures Config[addr].active ==> result == 77;
        ensures Config[addr] == old(Config[addr]);
    }
}
