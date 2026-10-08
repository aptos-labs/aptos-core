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

    public fun call_noncapturing(): u64 {
        let f = 0x1::from_bcs::from_bytes<|u64, u64|u64 has copy + drop>(x"05010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637303616464000000000000000000");
        f(3, 4)
    }

    public fun call_capturing(): u64 {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has copy + drop>(x"07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637303616464000100000000000000020700000000000000");
        f(5)
    }

    public fun call_capturing_twice(): u64 {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has copy + drop>(x"07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637303616464000100000000000000020700000000000000");
        f(5) + f(6)
    }

    public fun call_vector_capture(): u64 {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has copy + drop>(x"07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730574616c6c79000100000000000000050103010203");
        f(10)
    }

    public fun call_then_serialize(): vector<u8> {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has copy + drop>(x"07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730574616c6c79000100000000000000050103010203");
        let _ = f(10);
        bcs::to_bytes(&f)
    }

    public fun call_across_gc(): u64 {
        let f = 0x1::from_bcs::from_bytes<|u64|u64 has copy + drop>(x"07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730574616c6c79000100000000000000050103010203");
        0x0::test_utils::force_gc();
        let a = f(10);
        0x0::test_utils::force_gc();
        let b = f(20);
        a + b
    }
}

// RUN: execute 0x99::closure_bcs::call_noncapturing
// CHECK: results: 7

// RUN: execute 0x99::closure_bcs::call_capturing
// CHECK: results: 12

// RUN: execute 0x99::closure_bcs::call_capturing_twice
// CHECK: results: 25

// RUN: execute 0x99::closure_bcs::call_vector_capture
// CHECK: results: 13

// RUN: execute 0x99::closure_bcs::call_then_serialize
// CHECK: results: 0x07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730574616c6c79000100000000000000050103010203

// RUN: execute 0x99::closure_bcs::call_across_gc
// CHECK: results: 36
