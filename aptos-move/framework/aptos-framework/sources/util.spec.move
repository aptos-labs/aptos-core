spec aptos_framework::util {
    spec from_bytes<T>(bytes: vector<u8>): T {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_from_bytes<T>(bytes);
    }

    spec fun spec_from_bytes<T>(bytes: vector<u8>): T;
}
