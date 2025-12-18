//# publish
module 0xc0ffee::m {
    use std::vector;

    public entry fun borrow_read_and_mutate() {
        let values = vector::empty<u64>();
        vector::push_back(&mut values, 1);
        vector::push_back(&mut values, 2);

        let first = *vector::borrow(&values, 0);

        {
            let second = vector::borrow_mut(&mut values, 1);
            *second = first + 5;
        };

        let updated = *vector::borrow(&values, 1);
        assert!(updated == 6, 0);
    }
}

//# run 0xc0ffee::m::borrow_read_and_mutate
