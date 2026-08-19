// exclude_for: cvc5
module 0x42::bv_internal_aggregate {

    spec fun bytes_value(v: vector<u8>): u64;

    fun packs_bytes(v: &vector<u8>): u64 {
        let acc = 0u64;
        acc = acc | (v[0] as u64);
        acc
    }
    spec packs_bytes {
        pragma opaque;
        pragma bv_internal;
        ensures [abstract] result == bytes_value(v);
    }

    fun feeds_bitwise_vector(x: u8): u64 {
        let v = vector[x & 1];
        packs_bytes(&v)
    }
    spec feeds_bitwise_vector {
        ensures result == bytes_value(vector[x & 1]);
    }
}
