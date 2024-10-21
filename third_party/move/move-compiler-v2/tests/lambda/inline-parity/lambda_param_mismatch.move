module 0x42::LambdaParam {
    use std::vector;
    use std::signer;

    struct VectorExample has key {
        inner: vector<u64>
    }

    fun vector_for_each<Element>(v: vector<Element>, f: |Element|) {
        vector::reverse(&mut v); // We need to reverse the vector to consume it efficiently
        while (!vector::is_empty(&v)) {
            let e = vector::pop_back(&mut v);
            f(e);
        };
    }

    entry fun remove_vector(caller: &signer, input: vector<u64>) acquires VectorExample {
        let caller_add = signer::address_of(caller);
        let mut_ref = borrow_global_mut<VectorExample>(caller_add);
        vector_for_each(input, |item| {
            vector::remove(&mut mut_ref.inner, item)
        })
    }
}
