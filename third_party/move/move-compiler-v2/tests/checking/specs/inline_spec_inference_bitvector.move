module 0x42::bit_vector_infer {
    use std::vector;

    public fun new(_length: u64) {
        let counter = 1;
        if (counter > 0) {
            counter = counter - 1;
        };
        let bit_field = vector::empty();
        vector::push_back(&mut bit_field, false);
        spec {
            assert len(bit_field) == 0;
        };
    }
}
