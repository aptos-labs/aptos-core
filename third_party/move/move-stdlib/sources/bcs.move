/// Utility for converting a Move value to its binary representation in BCS (Binary Canonical
/// Serialization). BCS is the binary encoding for Move resources and other non-module values
/// published on-chain. See https://github.com/diem/bcs#binary-canonical-serialization-bcs for more
/// details on BCS.
module std::bcs {
    /// Return the binary representation of `v` in BCS (Binary Canonical Serialization) format
    public fun to_bytes<MoveValue>(v: &MoveValue): vector<u8> {
        native_load_layout<MoveValue>();
        native_to_bytes<MoveValue>(v)
    }

    native fun native_load_layout<MoveValue>();
    native fun native_to_bytes<MoveValue>(v: &MoveValue): vector<u8>;

    // ==============================
    // Module Specification
    spec module {} // switch to module documentation context

    spec module {
        /// Native function which is defined in the prover's prelude.
        native fun serialize<MoveValue>(v: &MoveValue): vector<u8>;
    }

    spec to_bytes<MoveValue>(v: &MoveValue): vector<u8> {
        pragma opaque;
        aborts_if false;
        ensures result == serialize(v);
    }

    spec native_load_layout<MoveValue>() {
        pragma opaque;
        aborts_if [abstract] false;
    }
}
