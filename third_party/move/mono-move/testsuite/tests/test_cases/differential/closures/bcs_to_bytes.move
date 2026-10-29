// RUN: publish
module 0x99::closure_bcs {
    use std::bcs;

    struct Point has copy, drop, store { x: u64, y: u8 }

    #[persistent]
    fun add(a: u64, b: u64): u64 {
        a + b
    }

    #[persistent]
    fun tally(data: vector<u8>, extra: u64): u64 {
        data.length() + extra
    }

    #[persistent]
    fun norm(p: Point, k: u64): u64 {
        p.x + (p.y as u64) + k
    }

    #[persistent]
    fun pick<T: drop>(a: T, b: u64): u64 {
        let _ = a;
        b
    }

    public fun ser_noncapturing(): vector<u8> {
        let f: |u64, u64|u64 has drop = add;
        bcs::to_bytes(&f)
    }

    public fun ser_capturing(): vector<u8> {
        let f: |u64|u64 has drop = |b| add(7, b);
        bcs::to_bytes(&f)
    }

    public fun ser_vector_capture(): vector<u8> {
        let data = vector[1u8, 2u8, 3u8];
        let f: |u64|u64 has drop = |k| tally(data, k);
        bcs::to_bytes(&f)
    }

    public fun ser_struct_capture(): vector<u8> {
        let p = Point { x: 5, y: 7 };
        let f: |u64|u64 has drop = |k| norm(p, k);
        bcs::to_bytes(&f)
    }

    public fun ser_generic(): vector<u8> {
        let a = true;
        let f: |u64|u64 has drop = |b| pick<bool>(a, b);
        bcs::to_bytes(&f)
    }

    public fun ser_generic_struct(): vector<u8> {
        let p = Point { x: 1, y: 2 };
        let f: |u64|u64 has drop = |b| pick<Point>(p, b);
        bcs::to_bytes(&f)
    }
}

// RUN: execute 0x99::closure_bcs::ser_noncapturing
// CHECK: results: 0x05010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637303616464000000000000000000

// RUN: execute 0x99::closure_bcs::ser_capturing
// CHECK: results: 0x07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637303616464000100000000000000020700000000000000

// RUN: execute 0x99::closure_bcs::ser_vector_capture
// CHECK: results: 0x07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f6263730574616c6c79000100000000000000050103010203

// RUN: execute 0x99::closure_bcs::ser_struct_capture
// CHECK: results: 0x07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f626373046e6f726d0001000000000000000600020201050000000000000007

// RUN: execute 0x99::closure_bcs::ser_generic
// CHECK: results: 0x07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f626373047069636b010001000000000000000001

// RUN: execute 0x99::closure_bcs::ser_generic_struct
// CHECK: results: 0x07010000000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f626373047069636b010700000000000000000000000000000000000000000000000000000000000000990b636c6f737572655f62637305506f696e740001000000000000000600020201010000000000000002
