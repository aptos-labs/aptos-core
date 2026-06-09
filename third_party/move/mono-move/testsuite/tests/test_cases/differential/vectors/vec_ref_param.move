// Vector operation through an `&mut vector<T>` reference parameter.

// RUN: publish
module 0xc0ffee::vec_ref_param {
    fun push_one(v: &mut vector<u64>, x: u64) {
        std::vector::push_back(v, x);
    }

    public fun push_then_pop(x: u64): u64 {
        let v = std::vector::empty<u64>();
        push_one(&mut v, x);
        std::vector::pop_back(&mut v)
    }
}

// RUN: execute 0xc0ffee::vec_ref_param::push_then_pop --args 0
// CHECK: results: 0

// RUN: execute 0xc0ffee::vec_ref_param::push_then_pop --args 42
// CHECK: results: 42
