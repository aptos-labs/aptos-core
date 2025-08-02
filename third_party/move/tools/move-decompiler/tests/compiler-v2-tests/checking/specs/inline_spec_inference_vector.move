module 0x42::bit_vector {
    use std::vector;

    public fun new(_length: u64) {
        let bit_field = vector::empty();
        spec {
            assert len(bit_field) == 0;
        };
        vector::push_back(&mut bit_field, false);
    }
}
