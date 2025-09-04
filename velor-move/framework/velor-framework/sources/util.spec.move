spec velor_framework::util {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: The address input bytes should be exactly 32 bytes long.
    /// Criticality: Low
    /// Implementation: The address_from_bytes function should assert if the length of the input bytes is 32.
    /// Enforcement: Verified via [high-level-req-1](address_from_bytes).
    ///</high-level-req>
    ///
    spec from_bytes<T>(bytes: vector<u8>): T {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_from_bytes<T>(bytes);
    }

    spec fun spec_from_bytes<T>(bytes: vector<u8>): T;

    spec address_from_bytes(bytes: vector<u8>): address {

        // This is an abstract specification and the soundness of this abstraction depends on the native function.
        // If length of address input bytes is not 32, the deserialization will fail. See indexer/src/utils.rs.
        /// [high-level-req-1]
        aborts_if [abstract] len(bytes) != 32;
    }
}
