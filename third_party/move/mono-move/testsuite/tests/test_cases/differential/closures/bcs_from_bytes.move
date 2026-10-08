// RUN: publish
module 0x1::from_bcs {
    public native fun from_bytes<T>(bytes: vector<u8>): T;
}

module 0x99::closure_bcs {
    use std::bcs;

    #[persistent]
    fun add(a: u64, b: u64): u64 {
        a + b
    }

    #[persistent]
    fun tally(data: vector<u8>, extra: u64): u64 {
        data.length() + extra
    }

    public fun rt_noncapturing(): vector<u8> {
        let f = 0x1::from_bcs::from_bytes<|u64, u64|u64 has drop>(x"05010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637303616464000000000000000000");
        bcs::to_bytes(&f)
    }

    public fun rt_capturing(): vector<u8> {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has drop>(x"07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637303616464000100000000000000020700000000000000");
        bcs::to_bytes(&f)
    }

    public fun rt_vector_capture(): vector<u8> {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has drop>(x"07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730574616c6c79000100000000000000050103010203");
        bcs::to_bytes(&f)
    }

    // The target does not exist, so the identity is interned but never loaded.
    public fun rt_unknown_target(): vector<u8> {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has drop>(x"05010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730c6e6f745f615f6d6574686f64000000000000000000");
        bcs::to_bytes(&f)
    }

    // Aborts: the capture count the mask implies disagrees with the sequence.
    public fun rt_capture_count_mismatch(): vector<u8> {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has drop>(x"09010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637303616464000100000000000000020700000000000000");
        bcs::to_bytes(&f)
    }
}

// RUN: execute 0x99::closure_bcs::rt_noncapturing
// CHECK: results: 0x05010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637303616464000000000000000000

// RUN: execute 0x99::closure_bcs::rt_capturing
// CHECK: results: 0x07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637303616464000100000000000000020700000000000000

// RUN: execute 0x99::closure_bcs::rt_vector_capture
// CHECK: results: 0x07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730574616c6c79000100000000000000050103010203

// RUN: execute 0x99::closure_bcs::rt_unknown_target
// CHECK: results: 0x05010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730c6e6f745f615f6d6574686f64000000000000000000

// RUN: execute 0x99::closure_bcs::rt_capture_count_mismatch
// CHECK: aborted: code 65537 in 0x1::from_bcs
