/// Utility for converting a Move value to its binary representation in BCS (Binary Canonical
/// Serialization). BCS is the binary encoding for Move resources and other non-module values
/// published on-chain. See https://github.com/aptos-labs/bcs#binary-canonical-serialization-bcs for more
/// details on BCS.
module std::bcs {
    /// Returns the binary representation of `v` in BCS (Binary Canonical Serialization) format.
    /// Aborts with `0x1c5` error code if serialization fails.
    native public fun to_bytes<MoveValue>(v: &MoveValue): vector<u8>;

    /// Returns the size of the binary representation of `v` in BCS (Binary Canonical Serialization) format.
    /// Aborts with `0x1c5` error code if there is a failure when calculating serialized size.
    native public fun serialized_size<MoveValue>(v: &MoveValue): u64;

    // ==============================
    // Module Specification
    spec module {} // switch to module documentation context

    spec module {
        /// Native function which is defined in the prover's prelude.
        native fun serialize<MoveValue>(v: &MoveValue): vector<u8>;
    }

    spec serialized_size<MoveValue>(v: &MoveValue): u64 {
        pragma opaque;
        ensures result == len(serialize(v));
    }
}
