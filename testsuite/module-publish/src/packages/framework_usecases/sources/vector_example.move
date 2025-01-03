
/// test speed of vector operations
module 0xABCD::vector_example {
    use std::vector;

    fun generate_vec(vec_len: u64, element_len: u64): vector<vector<u64>> {
        let elem = vector::empty<u64>();
        for (i in 0..element_len) {
            vector::push_back(&mut elem, i);
        };
        let vec = vector::empty();
        for (i in 0..vec_len) {
            let cur = elem;
            cur[0] = i;
            vector::push_back(&mut vec, cur);
        };
        vec
    }

    public entry fun test_trim_append(vec_len: u64, element_len: u64, index: u64, repeats: u64) {
        let vec = generate_vec(vec_len, element_len);

        for (i in 0..repeats) {
            let part = vector::trim(&mut vec, index);
            vector::append(&mut vec, part);
        };
    }

    public entry fun test_remove_insert(vec_len: u64, element_len: u64, index: u64, repeats: u64) {
        let vec = generate_vec(vec_len, element_len);

        for (i in 0..repeats) {
            let part = vector::remove(&mut vec, index);
            vector::insert(&mut vec, index, part);
        };
    }

    // public entry fun test_middle_range_move(vec_len: u64, element_len: u64, index: u64, move_len: u64, repeats: u64) {
    //     let vec1 = generate_vec(vec_len, element_len);
    //     let vec2 = generate_vec(vec_len, element_len);

    //     for (i in 0..repeats) {
    //         vector::move_range(&mut vec1, index, move_len, &mut vec2, index);
    //         vector::move_range(&mut vec2, index, move_len, &mut vec1, index);
    //     };
    // }
}
