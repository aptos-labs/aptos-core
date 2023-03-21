spec aptos_std::algebra {

    spec deserialize_internal<G, F>(bytes: &vector<u8>): (bool, u64) {
        pragma opaque;
    }

    spec add_internal<F>(handle_1: u64, handle_2: u64): u64 {
        pragma opaque;
    }

    spec serialize_internal<G, F>(handle: u64): vector<u8> {
        pragma opaque;
    }
}
