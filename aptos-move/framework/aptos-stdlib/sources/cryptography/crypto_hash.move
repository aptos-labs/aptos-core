module aptos_std::crypto_hash {

    /// 32 bytes represented by two u128 in little endian encoding, the same as BCS does
    struct HashValue has copy, drop, store{
        low: u128,
        high: u128,
    }

    public fun new_hash_value(low: u128, high: u128): HashValue {
        HashValue {
            low,
            high,
        }
    }

}
