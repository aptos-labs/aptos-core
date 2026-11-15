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
    spec single(s: &signer, counter: &mut u64): u64 {
        use 0x1::signer;
        pragma opaque = true;
        modifies R[signer::address_of(s)];
        ensures [inferred] result == result_of<local_write>(old(counter));
        ensures [inferred] publish<R>(signer::address_of(s), R{value: 1});
        ensures [inferred] ensures_of<local_write>(old(counter), result, counter);
        aborts_if [inferred] exists<R>(signer::address_of(s));
    }


    fun branched(s1: &signer, s2: &signer, s3: &signer, cond: bool, counter: &mut u64) {
        move_to(s1, R { value: 2 });
        if (cond) {
            move_to(s2, R { value: 3 });
            move_to(s3, R { value: 4 });
        };
        local_write(counter);
    }
    spec branched(s1: &signer, s2: &signer, s3: &signer, cond: bool, counter: &mut u64) {
        use 0x1::signer;
        pragma opaque = true;
        modifies R[signer::address_of(s3)];
        modifies R[signer::address_of(s2)];
        modifies R[signer::address_of(s1)];
        ensures [inferred] cond ==> {
            let a = signer::address_of(s3);
            S2.. |~ publish<R>(a, R{value: 4})
        };
        ensures [inferred] cond ==> {
            let a = signer::address_of(s2);
            S1..S2 |~ publish<R>(a, R{value: 3})
        };
        ensures [inferred] {
            let a = signer::address_of(s1);
            ..S1 |~ publish<R>(a, R{value: 2})
        };
        ensures [inferred] ensures_of<local_write>(old(counter), result_of<local_write>(old(counter)), counter);
        aborts_if [inferred] exists<R>(signer::address_of(s1));
        aborts_if [inferred] S2 |~ (cond && exists<R>(signer::address_of(s3)));
        aborts_if [inferred] S1 |~ (cond && exists<R>(signer::address_of(s2)));
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
    spec vector_after_branch(s1: &signer, s2: &signer, s3: &signer, cond: bool): vector<u64> {
        use 0x1::signer;
        pragma opaque = true;
        modifies R[signer::address_of(s3)];
        modifies R[signer::address_of(s2)];
        modifies R[signer::address_of(s1)];
        ensures [inferred] result == update(vec(42), 0, 43);
        ensures [inferred] cond ==> {
            let a = signer::address_of(s3);
            S2.. |~ publish<R>(a, R{value: 7})
        };
        ensures [inferred] cond ==> {
            let a = signer::address_of(s2);
            S1..S2 |~ publish<R>(a, R{value: 6})
        };
        ensures [inferred] {
            let a = signer::address_of(s1);
            ..S1 |~ publish<R>(a, R{value: 5})
        };
        aborts_if [inferred] exists<R>(signer::address_of(s1));
        aborts_if [inferred] !in_range(vec(42), 0);
        aborts_if [inferred] S2 |~ (cond && exists<R>(signer::address_of(s3)));
        aborts_if [inferred] S1 |~ (cond && exists<R>(signer::address_of(s2)));
    }


    fun option_after_write(s: &signer): u64 {
        move_to(s, R { value: 8 });
        let o = option::some(42);
        option::destroy_some(o)
    }
    spec option_after_write(s: &signer): u64 {
        use 0x1::option;
        use 0x1::signer;
        pragma opaque = true;
        modifies R[signer::address_of(s)];
        ensures [inferred] result == option::destroy_some<u64>(option::some<u64>(42));
        ensures [inferred] publish<R>(signer::address_of(s), R{value: 8});
        aborts_if [inferred] exists<R>(signer::address_of(s));
        aborts_if [inferred] aborts_of<option::destroy_some<u64>>(option::some<u64>(42));
    }


    fun map_after_write(s: &signer, m: &mut SimpleMap<u64, u64>, k: u64, v: u64) {
        move_to(s, R { value: 9 });
        simple_map::add(m, k, v);
        *simple_map::borrow_mut(m, &k) = 10;
    }
    spec map_after_write(s: &signer, m: &mut 0x1::simple_map::SimpleMap<u64, u64>, k: u64, v: u64) {
        use 0x1::signer;
        use 0x1::simple_map;
        pragma opaque = true;
        modifies R[signer::address_of(s)];
        ensures [inferred] m == simple_map::spec_set<u64, u64>(simple_map::spec_set<u64, u64>(old(m), k, v), k, 10);
        ensures [inferred] publish<R>(signer::address_of(s), R{value: 9});
        aborts_if [inferred] exists<R>(signer::address_of(s));
        aborts_if [inferred] simple_map::spec_aborts_add<u64, u64>(m, k, v);
        aborts_if [inferred] simple_map::spec_aborts_borrow<u64, u64>(simple_map::spec_set<u64, u64>(m, k, v), k);
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
    spec pure_key_after_publish(s: &signer): u64 {
        use 0x1::signer;
        pragma opaque = true;
        modifies R[signer::address_of(s)];
        ensures [inferred] result == pure_key<R>(42);
        ensures [inferred] publish<R>(signer::address_of(s), R{value: 10});
        aborts_if [inferred] exists<R>(signer::address_of(s));
    }


}
/*
Verification: Succeeded.
Mutation: Rejected by postcondition.
*/
