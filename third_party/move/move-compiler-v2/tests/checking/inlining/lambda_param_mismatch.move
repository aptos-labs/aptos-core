module 0x42::LambdaParam {
    use std::vector;
    use std::signer;

    struct VectorExample has key {
        inner: vector<u64>
    }

    entry fun remove_vector(caller: &signer, input: vector<u64>) acquires VectorExample {
        let caller_add = signer::address_of(caller);
        let mut_ref = borrow_global_mut<VectorExample>(caller_add);
        vector::for_each(input, |item| {
            vector::remove(&mut mut_ref.inner, item)
        })
    }
}
