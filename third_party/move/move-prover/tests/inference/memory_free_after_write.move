// A memory-free call after a global write must not move the backward memory
// cursor to a forward join label. The final publish must constrain final memory.
// inference-reject-mutation: move_to(s, R { value: 1 }); => move_to(s, R { value: 99 });
module 0x42::memory_free_after_write {
    use std::option;
    use std::vector;
    use aptos_std::simple_map::{Self, SimpleMap};

    struct R has key { value: u64 }

    fun local_write(counter: &mut u64): u64 {
        *counter = 1;
        1
    }
    spec local_write {
        pragma opaque;
        pragma inference = none;
        aborts_if false;
        ensures counter == 1;
        ensures result == 1;
    }

    fun single(s: &signer, counter: &mut u64): u64 {
        move_to(s, R { value: 1 });
        local_write(counter)
    }

    fun branched(s1: &signer, s2: &signer, s3: &signer, cond: bool, counter: &mut u64) {
        move_to(s1, R { value: 2 });
        if (cond) {
            move_to(s2, R { value: 3 });
            move_to(s3, R { value: 4 });
        };
        local_write(counter);
    }

    fun vector_after_branch(s1: &signer, s2: &signer, s3: &signer, cond: bool): vector<u64> {
        move_to(s1, R { value: 5 });
        if (cond) {
            move_to(s2, R { value: 6 });
            move_to(s3, R { value: 7 });
        };
        let v = vector[];
        vector::push_back(&mut v, 42);
        *vector::borrow_mut(&mut v, 0) = 43;
        v
    }

    fun option_after_write(s: &signer): u64 {
        move_to(s, R { value: 8 });
        let o = option::some(42);
        option::destroy_some(o)
    }

    fun map_after_write(s: &signer, m: &mut SimpleMap<u64, u64>, k: u64, v: u64) {
        move_to(s, R { value: 9 });
        simple_map::add(m, k, v);
        *simple_map::borrow_mut(m, &k) = 10;
    }

    fun pure_key<T: key>(n: u64): u64 { n }
    spec pure_key {
        pragma opaque;
        pragma inference = none;
        aborts_if false;
        ensures result == n;
    }

    fun pure_key_after_publish(s: &signer): u64 {
        move_to(s, R { value: 10 });
        pure_key<R>(42)
    }

}
