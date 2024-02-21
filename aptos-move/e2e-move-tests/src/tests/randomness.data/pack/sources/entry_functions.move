module 0x1::entry_functions {
    use aptos_framework::randomness;

    entry fun ok_if_not_annotated_and_not_using_randomness() {
        // Do nothing.
    }

    #[uses_randomness]
    entry fun ok_if_annotated_and_not_using_randomness() {
        // Do nothing.
    }

    entry fun fail_if_not_annotated_and_using_randomness() {
        let _ = randomness::u64_integer();
    }

    #[uses_randomness]
    entry fun ok_if_annotated_and_using_randomness() {
        let _ = randomness::u64_integer();
    }
}
