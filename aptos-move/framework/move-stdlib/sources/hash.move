/// Module which defines SHA hashes for byte vectors.
///
/// The functions in this module are natively declared both in the Move runtime
/// as in the Move prover's prelude.
module std::hash {
    use std::bcs;

    // TODO: move sip_hash into aptos_framework
    public fun sip_hash<MoveValue>(v: &MoveValue): u64 {
        let bytes = bcs::to_bytes(v);

        sip_hash_internal(bytes)
    }

    spec sip_hash { // TODO: temporary mockup.
        pragma opaque;
    }

    native fun sip_hash_internal(bytes: vector<u8>): u64;
    native public fun sha2_256(data: vector<u8>): vector<u8>;
    native public fun sha3_256(data: vector<u8>): vector<u8>;
}
