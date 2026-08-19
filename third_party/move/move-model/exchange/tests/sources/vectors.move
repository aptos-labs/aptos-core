module 0x42::vectors {
    use std::vector;

    fun exercise(first: u64): u64 {
        let v = vector[first, 20];
        vector::push_back(&mut v, 30);
        let last = vector::pop_back(&mut v);
        v[1] = 25;
        last + vector::length(&v) + *vector::borrow(&v, 0) + v[1]
    }
}
