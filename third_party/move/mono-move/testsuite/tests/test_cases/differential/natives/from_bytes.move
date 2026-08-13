// Differential test for `from_bcs::from_bytes` and `util::from_bytes`, which
// share one implementation. Neither module is in the bundled stdlib, so the
// natives are declared here.

// RUN: publish
module 0x1::from_bcs {
    public native fun from_bytes<T>(bytes: vector<u8>): T;
}
module 0x1::util {
    public native fun from_bytes<T>(bytes: vector<u8>): T;
}
module 0x1::create_signer {
    public native fun create_signer(addr: address): signer;
}
module 0x1::main {
    use std::bcs;

    struct Pair has drop {
        a: u64,
        b: bool,
    }

    public fun from_bcs_u64(): u64 {
        0x1::from_bcs::from_bytes<u64>(x"2a00000000000000")
    }

    // Deserialize a (non-generic) struct and read a field back.
    public fun from_bcs_struct(): u64 {
        let p: Pair = 0x1::from_bcs::from_bytes<Pair>(x"070000000000000001");
        p.a
    }

    public fun util_u64(): u64 {
        0x1::util::from_bytes<u64>(x"2a00000000000000")
    }

    public fun from_bcs_bool(): bool {
        0x1::from_bcs::from_bytes<bool>(x"01")
    }

    public fun from_bcs_address(): address {
        0x1::from_bcs::from_bytes<address>(
            x"0000000000000000000000000000000000000000000000000000000000000001"
        )
    }

    // Round-trip through bcs::to_bytes for a byte vector.
    public fun roundtrip_bytes(): vector<u8> {
        0x1::from_bcs::from_bytes<vector<u8>>(bcs::to_bytes(&b"hello"))
    }

    // Too few bytes for a u64.
    public fun truncated(): u64 {
        0x1::from_bcs::from_bytes<u64>(x"2a")
    }

    // Trailing bytes after a complete u64.
    public fun trailing(): u64 {
        0x1::from_bcs::from_bytes<u64>(x"2a0000000000000000")
    }

    // A signer serializes to its address but must never deserialize.
    public fun signer_roundtrip() {
        let s = 0x1::create_signer::create_signer(@0x123);
        0x1::from_bcs::from_bytes<signer>(bcs::to_bytes(&s));
    }

    // A signer nested in a vector is rejected too.
    public fun signer_in_vector() {
        0x1::from_bcs::from_bytes<vector<signer>>(
            x"010000000000000000000000000000000000000000000000000000000000000000"
        );
    }
}

// RUN: execute 0x1::main::from_bcs_u64
// CHECK: results: 42

// RUN: execute 0x1::main::from_bcs_struct
// CHECK: results: 7

// RUN: execute 0x1::main::util_u64
// CHECK: results: 42

// RUN: execute 0x1::main::from_bcs_bool
// CHECK: results: true

// RUN: execute 0x1::main::from_bcs_address
// CHECK: results: 0x1

// RUN: execute 0x1::main::roundtrip_bytes
// CHECK: results: 0x68656c6c6f

// Malformed input aborts with EFROM_BYTES on both VMs.

// RUN: execute 0x1::main::truncated
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::trailing
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::signer_roundtrip
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::signer_in_vector
// CHECK: aborted: code 65537 in 0x1::from_bcs
