// Copyright © Aptos Foundation
// Tests that the compiler validates closures stored into struct fields against
// `reads_of`/`modifies_of` declarations on those fields. Mirrors the function
// parameter validation in access_of_errors.move.

module 0x42::stored_fun_access_errors {
    struct Counter has key { value: u64 }
    struct Config  has key { active: bool }

    // =========================================================================
    // Library: opaque closures that access resources
    // =========================================================================

    #[persistent]
    fun read_counter(addr: address): u64 {
        Counter[addr].value
    }
    spec read_counter {
        pragma opaque;
        ensures result == Counter[addr].value;
        aborts_if !exists<Counter>(addr);
    }

    #[persistent]
    fun read_config(addr: address): bool {
        Config[addr].active
    }
    spec read_config {
        pragma opaque;
        ensures result == Config[addr].active;
        aborts_if !exists<Config>(addr);
    }

    #[persistent]
    fun read_both(addr: address): u64 {
        if (Config[addr].active) { Counter[addr].value } else { 0 }
    }
    spec read_both {
        pragma opaque;
        aborts_if !exists<Config>(addr);
        aborts_if Config[addr].active && !exists<Counter>(addr);
    }

    #[persistent]
    fun write_counter(addr: address) {
        Counter[addr].value = Counter[addr].value + 1;
    }
    spec write_counter {
        pragma opaque;
        modifies Counter[addr];
        ensures Counter[addr].value == old(Counter[addr].value) + 1;
        aborts_if !exists<Counter>(addr);
    }

    // =========================================================================
    // 1. reads_of too narrow: closure accesses undeclared resource
    // =========================================================================

    struct NarrowReader has key, drop {
        f: |address|u64 has copy+store+drop,
    }
    spec NarrowReader {
        reads_of<f> Counter;
    }

    /// Pack with read_both which reads Counter AND Config — but only Counter is declared
    fun create_narrow_reader(): NarrowReader {
        NarrowReader { f: read_both } // error: accesses Config not in reads_of
    }

    /// Pack with read_counter which only reads Counter — should verify
    fun create_narrow_reader_ok(): NarrowReader {
        NarrowReader { f: read_counter }
    }

    // =========================================================================
    // 2. reads_of but closure writes: write to read-only resource
    // =========================================================================

    struct ReadOnlyCounter has key, drop {
        f: |address| has copy+store+drop,
    }
    spec ReadOnlyCounter {
        reads_of<f> Counter;
    }

    /// Pack with write_counter which modifies Counter — only reads_of declared
    fun create_read_only_writer(): ReadOnlyCounter {
        ReadOnlyCounter { f: write_counter } // error: writes Counter but only reads_of
    }

    // =========================================================================
    // 3. modifies_of too narrow: closure modifies undeclared resource
    // =========================================================================

    struct NarrowWriter has key, drop {
        f: |address| has copy+store+drop,
    }
    spec NarrowWriter {
        modifies_of<f>(a: address) Config[a];
    }

    /// Pack with write_counter which modifies Counter — but only Config is declared
    fun create_narrow_writer(): NarrowWriter {
        NarrowWriter { f: write_counter } // error: accesses Counter not in modifies_of
    }

    // =========================================================================
    // 4. reads_of<f> * wildcard must still reject writes
    // =========================================================================

    struct WildcardReader has key, drop {
        f: |address| has copy+store+drop,
    }
    spec WildcardReader {
        reads_of<f> *;
    }

    /// Pack with write_counter into reads_of * struct: should fail because
    /// reads_of * does not grant write permission
    fun create_wildcard_reader_writer(): WildcardReader {
        WildcardReader { f: write_counter } // error: writes Counter but only reads_of
    }

    // =========================================================================
    // 5. Compliant packing should verify
    // =========================================================================

    struct WideReader has key, drop {
        f: |address|u64 has copy+store+drop,
    }
    spec WideReader {
        reads_of<f> Counter, Config;
    }

    /// Pack with read_both which reads Counter AND Config — both declared
    fun create_wide_reader_ok(): WideReader {
        WideReader { f: read_both }
    }

    /// Pack with read_counter which only reads Counter — subset, should verify
    fun create_wide_reader_subset(): WideReader {
        WideReader { f: read_counter }
    }
}
