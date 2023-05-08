module self::parallel_vector {
    friend self::tokens;
    friend self::bugs;

    use self::big_vector;
    use self::big_vector::BigVector;
    use aptos_std::table::{Self, Table};

    struct ParallelVector<T: store> has key {
        vectors: Table<u64, BigVector<T>>,
        parallelism: u64,
    }

    public(friend) fun create<T: store>(publisher: &signer, parallelism: u64, bucket_size: u64) {
        let i = 0;
        let vectors = table::new<u64, BigVector<T>>();
        while (i < parallelism) {
            let vec = big_vector::new<T>(bucket_size);
            table::add(&mut vectors, i, vec);
            i = i + 1;
        };
        move_to(publisher, ParallelVector { vectors, parallelism })
    }

    public(friend) fun pop_back_index<T: store>(
        parallel_vector_holder: address,
        index: u64
    ): T acquires ParallelVector {
        let parallel_vector = borrow_global_mut<ParallelVector<T>>(parallel_vector_holder);
        let bucket = index % parallel_vector.parallelism;

        // TODO: something like if the largest vector is more than X big than smallest, pop from largest instead of bucket
        // TODO: handle when a vector is empty

        let vec = table::borrow_mut(&mut parallel_vector.vectors, bucket);
        big_vector::pop_back(vec)
    }

    public(friend) fun push_back<T: store>(
        parallel_vector_holder: address,
        item: T,
        index: u64
    ) acquires ParallelVector {
        let parallel_vector = borrow_global_mut<ParallelVector<T>>(parallel_vector_holder);
        let bucket = index % parallel_vector.parallelism;

        let vec = table::borrow_mut(&mut parallel_vector.vectors, bucket);
        big_vector::push_back(vec, item);
    }

    // This is expensive, but is only called when filling the PV with tokens
    public(friend) fun length<T: store>(parallel_vector_holder: address): u64 acquires ParallelVector {
        let parallel_vector = borrow_global_mut<ParallelVector<T>>(parallel_vector_holder);
        let i = 0;
        let sum: u64 = 0;
        while (i < parallel_vector.parallelism) {
            sum = sum + big_vector::length(table::borrow(&parallel_vector.vectors, i));
            i = i + 1;
        };
        sum
    }

    #[test(publisher = @self)]
    fun test_parallel(
        publisher: signer,
    ) acquires ParallelVector {
        use aptos_framework::account;
        use std::signer;

        account::create_account_for_test(@self);

        let holder = signer::address_of(&publisher);
        create<u64>(&publisher, 3, 3);

        push_back<u64>(holder, 1, 0);
        assert!(length<u64>(holder) == 1, 0);

        push_back<u64>(holder, 2, 0);
        push_back<u64>(holder, 3, 2);
        push_back<u64>(holder, 4, 1);
        push_back<u64>(holder, 5, 2);
        assert!(length<u64>(holder) == 5, 1);
    }
}
