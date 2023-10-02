spec aptos_framework::util {
    spec from_bytes<T>(bytes: vector<u8>): T {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_from_bytes<T>(bytes);
    }

    spec fun spec_from_bytes<T>(bytes: vector<u8>): T;

    spec address_from_bytes(bytes: vector<u8>): address {

        // This is an abstract specification and the soundness of this abstraction depends on the native function.
        // If length of address input bytes is not 32, the deserialization will fail. See indexer/src/utils.rs.
        aborts_if [abstract] len(bytes) != 32;
    }
}
