/// Non-cryptographic hashes
module aptos_std::aptos_hash {
    use std::bcs;

    native public fun sip_hash(bytes: vector<u8>): u64;

    public fun sip_hash_from_value<MoveValue>(v: &MoveValue): u64 {
        let bytes = bcs::to_bytes(v);

        sip_hash(bytes)
    }
}
