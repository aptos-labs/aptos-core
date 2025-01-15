/// Utility for converting a Move value to its binary representation in BCS (Binary Canonical
/// Serialization). BCS is the binary encoding for Move resources and other non-module values
/// published on-chain. See https://github.com/aptos-labs/bcs#binary-canonical-serialization-bcs for more
/// details on BCS.
module std::bcs {
    use std::option::Option;

    /// Note: all natives would fail if the MoveValue contains a permissioned signer in it.

    /// Returns the binary representation of `v` in BCS (Binary Canonical Serialization) format.
    /// Aborts with `0x1c5` error code if serialization fails.
    native public fun to_bytes<MoveValue>(v: &MoveValue): vector<u8>;

    /// Returns the size of the binary representation of `v` in BCS (Binary Canonical Serialization) format.
    /// Aborts with `0x1c5` error code if there is a failure when calculating serialized size.
    native public fun serialized_size<MoveValue>(v: &MoveValue): u64;

    /// If the type has known constant (always the same, independent of instance) serialized size
    /// in BCS (Binary Canonical Serialization) format, returns it, otherwise returns None.
    /// Aborts with `0x1c5` error code if there is a failure when calculating serialized size.
    ///
    /// Note:
    /// For some types it might not be known they have constant size, and function might return None.
    /// For example, signer appears to have constant size, but it's size might change.
    /// If this function returned Some() for some type before - it is guaranteed to continue returning Some().
    /// On the other hand, if function has returned None for some type,
    /// it might change in the future to return Some() instead, if size becomes "known".
    native public fun constant_serialized_size<MoveValue>(): Option<u64>;

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

    spec constant_serialized_size {
        // TODO: temporary mockup.
        pragma opaque;
    }
}
