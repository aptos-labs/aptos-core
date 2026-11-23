// Reading a vector through a reference (`*src`) makes an independent copy: each owner is
// then written in place, and a shared buffer would show both writes in both vectors. The
// golden pins how the read lowers.

// RUN: publish --print(micro-ops)
module 0x42::readref_copy {
    use std::vector;

    fun read(src: &vector<u64>): vector<u64> { *src }

    public fun independent(): (vector<u64>, vector<u64>) {
        let original = vector[1, 2];
        let copied = read(&original);
        *vector::borrow_mut(&mut copied, 0) = 9;
        *vector::borrow_mut(&mut original, 1) = 8;
        (original, copied)
    }
}

// RUN: execute 0x42::readref_copy::independent
// CHECK: results: [1, 8], [9, 2]
