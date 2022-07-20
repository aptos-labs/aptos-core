/// Module which defines SHA hashes for byte vectors.
///
/// The functions in this module are natively declared both in the Move runtime
/// as in the Move prover's prelude.
module std::hash {
    // TODO: move sip_hash into aptos_framework
    native public fun sip_hash<MoveValue>(v: &MoveValue): u64;
    native public fun sha2_256(data: vector<u8>): vector<u8>;
    native public fun sha3_256(data: vector<u8>): vector<u8>;
}
