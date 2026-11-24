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
module 0x99::closure_bcs {
    #[persistent]
    fun add(a: u64, b: u64): u64 {
        a + b
    }

    #[persistent]
    fun tally(data: vector<u8>, extra: u64): u64 {
        data.length() + extra
    }
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

    public fun closure_noncapturing(): vector<u8> {
        let f = 0x1::from_bcs::from_bytes<|u64, u64|u64 has drop>(x"05010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637303616464000000000000000000");
        bcs::to_bytes(&f)
    }

    public fun closure_capturing(): vector<u8> {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has drop>(x"07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637303616464000100000000000000020700000000000000");
        bcs::to_bytes(&f)
    }

    public fun closure_vector_capture(): vector<u8> {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has drop>(x"07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730574616c6c79000100000000000000050103010203");
        bcs::to_bytes(&f)
    }

    // The target does not exist, so the identity is interned but never loaded.
    public fun closure_unknown_target(): vector<u8> {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has drop>(x"05010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730c6e6f745f615f6d6574686f64000000000000000000");
        bcs::to_bytes(&f)
    }

    // Aborts: the capture count the mask implies disagrees with the sequence.
    public fun closure_capture_count_mismatch(): vector<u8> {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has drop>(x"09010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637303616464000100000000000000020700000000000000");
        bcs::to_bytes(&f)
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

// A deserialized closure serializes back to the bytes it was read from.

// RUN: execute 0x1::main::closure_noncapturing
// CHECK: results: 0x05010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637303616464000000000000000000

// RUN: execute 0x1::main::closure_capturing
// CHECK: results: 0x07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637303616464000100000000000000020700000000000000

// RUN: execute 0x1::main::closure_vector_capture
// CHECK: results: 0x07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730574616c6c79000100000000000000050103010203

// RUN: execute 0x1::main::closure_unknown_target
// CHECK: results: 0x05010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730c6e6f745f615f6d6574686f64000000000000000000

// RUN: execute 0x1::main::closure_capture_count_mismatch
// CHECK: aborted: code 65537 in 0x1::from_bcs
