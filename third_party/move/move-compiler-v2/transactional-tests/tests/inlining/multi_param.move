//# publish
module 0x42::Test {
    use 0x1::vector as V;

    // Can't use that from V because of precompiled import in transactional tests
    // TODO: we need to fix this
    public inline fun for_each_ref_mut<Element>(v: &mut vector<Element>, f: |&mut Element|) {
        let i = 0;
        while (i < V::length(v)) {
            f(V::borrow_mut(v, i));
            i = i + 1
        }
    }
    struct Elem<K, V> has drop { k: K, v: V }

    // Checks a multi-mutality scenario.
    inline fun elem_for_each_ref<K,V>(v: &mut vector<Elem<K,V>>, f: |&K, &mut V|u64): u64 {
        let result = 0;
        for_each_ref_mut(v, |elem| {
            let elem: &mut Elem<K, V> = elem; // Checks whether scoping is fine
            result = result + f(&elem.k, &mut elem.v);
        });
        result
    }

    public fun test() {
        assert!(elem_for_each_ref(&mut vector[Elem{k:1, v:2}], |x,y| *x + *y) == 3, 0)
    }
}

//# run 0x42::Test::test
