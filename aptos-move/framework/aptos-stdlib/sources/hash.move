/// Non-cryptographic hashes
module aptos_std::aptos_hash {
    use std::bcs;

    native public fun sip_hash(bytes: vector<u8>): u64;

    public fun sip_hash_from_value<MoveValue>(v: &MoveValue): u64 {
        let bytes = bcs::to_bytes(v);

        sip_hash(bytes)
    }

    spec sip_hash_from_value {
        // TODO: temporary mockup.
        pragma opaque;
    }

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
