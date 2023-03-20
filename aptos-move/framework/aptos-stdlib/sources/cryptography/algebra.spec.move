spec aptos_std::algebra {

    spec deserialize_internal<S, F>(bytes: &vector<u8>): (bool, u64) {
        pragma opaque;
    }

    spec field_add_internal<F>(handle_1: u64, handle_2: u64): u64 {
        pragma opaque;
    }

    spec serialize_internal<S, F>(handle: u64): vector<u8> {
        pragma opaque;
    }
}
