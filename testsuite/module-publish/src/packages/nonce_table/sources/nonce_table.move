module 0xABCD::nonce_table {
    use aptos_framework::account;
    use aptos_std::table::{Self, Table};
    use aptos_std::vector;
    use aptos_std::timestamp;
    use aptos_std::aptos_hash::sip_hash_from_value;
    use aptos_std::ordered_map::{Self, OrderedMap};
    use aptos_std::big_ordered_map::{Self, BigOrderedMap};
    use aptos_std::math64::min;
    use aptos_std::simple_map::{Self,SimpleMap};

    // struct NonceEntry has copy, drop, store {
    //     sender_address: address,
    //     nonce: u64,
    //     txn_expiration_time: u64,
    // }

    // struct Bucket has copy, drop, store {
    //     lowest_expiration_time: u64,
    //     nonces: vector<NonceEntry>,
    // }

    // struct Bucket has copy, drop, store {
    //     nonces: vector<NonceEntry>,
    // }

    // struct Bucket has copy, drop, store {
    //     nonces: SimpleMap<NonceKey, u64>,
    // }

    // struct Bucket has copy, drop, store {
    //     nonces: OrderedMap<NonceKey, u64>,
    // }

    // struct Bucket has store {
    //     nonces: BigOrderedMap<NonceKey, u64>,
    // }

    // struct NonceKey has copy, drop, store {
    //     sender_address: address,
    //     nonce: u64,
    // }

    // struct NonceHistory has key {
    //     // Key = sip_hash(NonceKey) % 200000
    //     // Value = Bucket { lowest_expiration_time: u64, nonces: vector<NonceEntry> }
    //     nonce_table: Table<u64, Bucket>,
    //     next_key: u64,
    // }

    // struct NonceHistory has key {
    //     // Bucket Index = hash(NonceKey) % 100k.
    //     // Bucket = OrderedMap<NonceKey, transaction_expiration_time>
    //     table_1: Table<u64, OrderedMap<NonceKey, u64>>,
    // }

    struct NonceHistorySignerCap has key {
        signer_cap: account::SignerCapability,
    }

    public(friend) entry fun initialize_table(publisher: &signer) {
        let table = table::new();
        let nonce_history = NonceHistory {
            nonce_table: table,
            next_key: 0,
        };
        move_to<NonceHistory>(publisher, nonce_history);
    }


    // public(friend) entry fun initialize(publisher: &signer) {
    //     let table_1 = table::new();
    //     let nonce_history = NonceHistory {
    //         table_1,
    //     };

    //     move_to<NonceHistory>(publisher, nonce_history);
    // }

    // // Returns true if the nonce is successfully inserted.
    // // Returns false if the nonce already exists.
    // public(friend) fun insert_nonce(
    //     publisher: address,
    //     sender_address: address,
    //     nonce: u64,
    //     txn_expiration_time: u64,
    // ) acquires NonceHistory {
    //     let nonce_history = borrow_global_mut<NonceHistory>(publisher);
    //     let nonce_key = NonceKey {
    //         sender_address,
    //         nonce,
    //     };
    //     let hash = sip_hash_from_value(&nonce_key);
    //     let index = hash % 200000;
    //     if (!table::contains(&nonce_history.table_1, index)) {
    //         table::add(&mut nonce_history.table_1, index, ordered_map::new());
    //     };
    //     ordered_map::add(table::borrow_mut(&mut nonce_history.table_1, index), nonce_key, txn_expiration_time)
    // }

    // public(friend) fun nonce_exists(
    //     publisher: address,
    //     sender_address: address,
    //     nonce: u64,
    //     txn_expiration_time: u64,
    // ): bool acquires NonceHistory {
    //     let nonce_key = NonceKey {
    //         sender_address,
    //         nonce,
    //     };
    //     let hash = sip_hash_from_value(&nonce_key);
    //     let index = hash % 200000;
    //     let nonce_history = borrow_global<NonceHistory>(publisher);
    //     if (table::contains(&nonce_history.table_1, index)) {
    //         if (ordered_map::contains(table::borrow(&nonce_history.table_1, index), &nonce_key)) {
    //             return true;
    //         }
    //     };
    //     false
    // }

    // public entry fun check_and_insert_nonce(
    //     publisher: address,
    //     sender_address: address,
    //     nonce: u64,
    //     txn_expiration_time: u64,
    // ) acquires NonceHistory {
    //     let nonce_key = NonceKey {
    //         sender_address,
    //         nonce,
    //     };
    //     let hash = sip_hash_from_value(&nonce_key);
    //     let index = hash % 200000;
    //     let nonce_history = borrow_global_mut<NonceHistory>(publisher);
    //     if (table::contains(&nonce_history.table_1, index)) {
    //         if (ordered_map::contains(table::borrow(&nonce_history.table_1, index), &nonce_key)) {
    //             return
    //         }
    //     } else {
    //         table::add(&mut nonce_history.table_1, index, ordered_map::new());
    //     };
    //     ordered_map::add(table::borrow_mut(&mut nonce_history.table_1, index), nonce_key, txn_expiration_time);
    // }

    // public entry fun check_and_insert_nonce(
    //     publisher: address,
    //     sender_address: address,
    //     nonce: u64,
    //     txn_expiration_time: u64,
    // ) acquires NonceHistory {
    //     let nonce_key = NonceKey {
    //         sender_address,
    //         nonce,
    //     };
    //     let hash = sip_hash_from_value(&nonce_key);
    //     let index = hash % 200000;
    //     let nonce_history = borrow_global_mut<NonceHistory>(publisher);
    //     if (table::contains(&nonce_history.table_1, index)) {
    //         if (ordered_map::contains(table::borrow(&nonce_history.table_1, index), &nonce_key)) {
    //             return
    //         }
    //     } else {
    //         table::add(&mut nonce_history.table_1, index, ordered_map::new());
    //     };
    //     ordered_map::add(table::borrow_mut(&mut nonce_history.table_1, index), nonce_key, txn_expiration_time);
    // }


    // 249.2 microseconds per call
    // public(friend) entry fun check_and_insert_nonce(
    //     publisher: address,
    //     sender_address: address,
    //     nonce: u64,
    //     txn_expiration_time: u64,
    // ) acquires NonceHistory {
    //     let nonce_history = borrow_global_mut<NonceHistory>(publisher);
    //     let nonce_entry = NonceEntry {
    //         sender_address,
    //         nonce,
    //         txn_expiration_time,
    //     };
    //     let nonce_key = NonceKey {
    //         sender_address,
    //         nonce,
    //     };
    //     let index = sip_hash_from_value(&nonce_key) % 200000;
    //     if (!table::contains(&nonce_history.nonce_table, index)) {
    //         let nonces = vector::empty();
    //         vector::push_back(&mut nonces, nonce_entry);
    //         table::add(&mut nonce_history.nonce_table, index, Bucket {
    //             lowest_expiration_time: txn_expiration_time,
    //             nonces: nonces,
    //         });
    //         return;
    //     };
    //     let bucket = table::borrow_mut(&mut nonce_history.nonce_table, index);
    //     if (vector::contains(&bucket.nonces, &nonce_entry)) {
    //         return;
    //     };
    //     let current_time = timestamp::now_seconds();
    //     if (current_time <= bucket.lowest_expiration_time) {
    //         // None of the nonces are expired. Just insert the nonce.
    //         vector::push_back(&mut bucket.nonces, nonce_entry);

    //         // Question: Is there a better way to do this?
    //         table::borrow_mut(&mut nonce_history.nonce_table, index).lowest_expiration_time = min(bucket.lowest_expiration_time, txn_expiration_time);
    //     } else {
    //         // There is an expired nonce. Remove the expired nonces.
    //         let new_bucket = Bucket {
    //             lowest_expiration_time: txn_expiration_time,
    //             nonces: vector::empty(),
    //         };
    //         let len = vector::length(&bucket.nonces);
    //         let i = 0;
    //         while (i < len) {
    //             let nonce_entry = vector::borrow(&bucket.nonces, i);
    //             if (current_time <= nonce_entry.txn_expiration_time) {
    //                 vector::push_back(&mut new_bucket.nonces, *nonce_entry);
    //                 new_bucket.lowest_expiration_time = min(new_bucket.lowest_expiration_time, nonce_entry.txn_expiration_time);
    //             };
    //             i = i + 1;
    //         };
    //         *table::borrow_mut(&mut nonce_history.nonce_table, index) = new_bucket;
    //     };
    // }

    // 247.2 microseconds per call
    // public(friend) entry fun check_and_insert_nonce(
    //     publisher: address,
    //     sender_address: address,
    //     nonce: u64,
    //     txn_expiration_time: u64,
    // ) acquires NonceHistory {
    //     let nonce_history = borrow_global_mut<NonceHistory>(publisher);
    //     let nonce_entry = NonceEntry {
    //         sender_address,
    //         nonce,
    //         txn_expiration_time,
    //     };
    //     let nonce_key = NonceKey {
    //         sender_address,
    //         nonce,
    //     };
    //     let index = sip_hash_from_value(&nonce_key) % 200000;
    //     if (!table::contains(&nonce_history.nonce_table, index)) {
    //         let nonces = vector::empty();
    //         vector::push_back(&mut nonces, nonce_entry);
    //         table::add(&mut nonce_history.nonce_table, index, Bucket {
    //             nonces: nonces,
    //         });
    //         return;
    //     };
    //     let bucket = table::borrow_mut(&mut nonce_history.nonce_table, index);
    //     if (vector::contains(&bucket.nonces, &nonce_entry)) {
    //         return;
    //     };
    //     let current_time = timestamp::now_seconds();
        
    //     let new_bucket = Bucket {
    //         nonces: vector::empty(),
    //     };

    //     let len = vector::length(&bucket.nonces);
    //     let i = 0;
    //     while (i < len) {
    //         let nonce_entry = vector::borrow(&bucket.nonces, i);
    //         if (current_time <= nonce_entry.txn_expiration_time) {
    //             vector::push_back(&mut new_bucket.nonces, *nonce_entry);
    //         };
    //         i = i + 1;
    //     };
    //     vector::push_back(&mut new_bucket.nonces, nonce_entry);
    //     *table::borrow_mut(&mut nonce_history.nonce_table, index) = new_bucket;
    // }

    // 247.2 microseconds per call for 200k buckets, 3 million transactions
    // 248.2 microseconds per call, 5332 gas/sec, 0.40 exe gas, 0.92 io gas for 10k buckets, 3 million transactions
    // 248.3 microseconds per call, 5331 gas/sec, 0.40 exe gas, 0.92 io gas for 20k buckets, 3 million transactions
    // public(friend) entry fun check_and_insert_nonce(
    //     publisher: address,
    //     sender_address: address,
    //     nonce: u64,
    //     txn_expiration_time: u64,
    // ) acquires NonceHistory {
    //     let nonce_history = borrow_global_mut<NonceHistory>(publisher);
    //     let nonce_entry = NonceEntry {
    //         sender_address,
    //         nonce,
    //         txn_expiration_time,
    //     };
    //     let nonce_key = NonceKey {
    //         sender_address,
    //         nonce,
    //     };
    //     let index = sip_hash_from_value(&nonce_key) % 20000;
    //     if (!table::contains(&nonce_history.nonce_table, index)) {
    //         let nonces = vector::empty();
    //         vector::push_back(&mut nonces, nonce_entry);
    //         table::add(&mut nonce_history.nonce_table, index, Bucket {
    //             nonces: nonces,
    //         });
    //         return;
    //     };
    //     let bucket = table::borrow_mut(&mut nonce_history.nonce_table, index);
    //     if (vector::contains(&bucket.nonces, &nonce_entry)) {
    //         return;
    //     };
    //     let current_time = timestamp::now_seconds();
        
    //     let len = vector::length(&bucket.nonces);
    //     if (len > 5) {
    //         let new_bucket = Bucket {
    //             nonces: vector::empty(),
    //         };

    //         let i = 0;
    //         while (i < len) {
    //             let nonce_entry = vector::borrow(&bucket.nonces, i);
    //             if (current_time <= nonce_entry.txn_expiration_time) {
    //                 vector::push_back(&mut new_bucket.nonces, *nonce_entry);
    //             };
    //             i = i + 1;
    //         };
    //         vector::push_back(&mut new_bucket.nonces, nonce_entry);
    //         *table::borrow_mut(&mut nonce_history.nonce_table, index) = new_bucket;
    //     } else {
    //         vector::push_back(&mut bucket.nonces, nonce_entry);
    //     }
    // }

    // 437.2 microseconds per call, 3239 gas/sec, 0.50 exe gas, 0.92 io gas for 200k buckets, 3 million transactions
    // public(friend) entry fun check_and_insert_nonce(
    //     publisher: address,
    //     sender_address: address,
    //     nonce: u64,
    //     txn_expiration_time: u64,
    // ) acquires NonceHistory {
    //     let nonce_history = borrow_global_mut<NonceHistory>(publisher);
    //     let nonce_key = NonceKey {
    //         sender_address,
    //         nonce,
    //     };
    //     let index = sip_hash_from_value(&nonce_key) % 200000;
    //     if (!table::contains(&nonce_history.nonce_table, index)) {
    //         let nonces = ordered_map::new();
    //         ordered_map::add(&mut nonces, nonce_key, txn_expiration_time);
    //         table::add(&mut nonce_history.nonce_table, index, Bucket {
    //             nonces: nonces,
    //         });
    //         return;
    //     };
    //     let bucket = table::borrow_mut(&mut nonce_history.nonce_table, index);
    //     if (ordered_map::contains(&bucket.nonces, &nonce_key)) {
    //         return;
    //     };

    //     let current_time = timestamp::now_seconds();
    //     let len = ordered_map::length(&bucket.nonces);
    //     if (len > 5) {
    //         let new_bucket = Bucket {
    //             nonces: ordered_map::new(),
    //         };
    //         let i = 0;
    //         while (i < len) {
    //             let (cur_nonce_key, cur_txn_expiration_time) = ordered_map::pop_front(&mut bucket.nonces);
    //             if (current_time <= cur_txn_expiration_time) {
    //                 ordered_map::add(&mut new_bucket.nonces, cur_nonce_key, cur_txn_expiration_time);
    //             };
    //             i = i + 1;
    //         };
    //         ordered_map::add(&mut new_bucket.nonces, nonce_key, txn_expiration_time);
    //         *table::borrow_mut(&mut nonce_history.nonce_table, index) = new_bucket;
    //     } else {
    //         ordered_map::add(&mut bucket.nonces, nonce_key, txn_expiration_time);
    //     }
    // }


    // without upsert - 1778.9 microseconds per call, 1118 gas/sec, 1.07 exe gas, 0.92 io gas for 200k buckets, 3 million transactions
    // with upsert - 
    // public(friend) entry fun check_and_insert_nonce(
    //     publisher: address,
    //     sender_address: address,
    //     nonce: u64,
    //     txn_expiration_time: u64,
    // ) acquires NonceHistory {
    //     let nonce_history = borrow_global_mut<NonceHistory>(publisher);
    //     let nonce_key = NonceKey {
    //         sender_address,
    //         nonce,
    //     };
    //     let index = sip_hash_from_value(&nonce_key) % 200000;
    //     if (!table::contains(&nonce_history.nonce_table, index)) {
    //         let nonces = big_ordered_map::new();
    //         big_ordered_map::add(&mut nonces, nonce_key, txn_expiration_time);
    //         table::add(&mut nonce_history.nonce_table, index, Bucket {
    //             nonces: nonces,
    //         });
    //         return;
    //     };
    //     let bucket = table::borrow_mut(&mut nonce_history.nonce_table, index);
    //     big_ordered_map::upsert(&mut bucket.nonces, nonce_key, txn_expiration_time);
    // }

    // 232.0 microseconds per call, 5666 gas/sec, 0.39 exe gas, 0.92 io gas for 200k buckets, 3 million transactions
    // public(friend) entry fun check_and_insert_nonce(
    //     publisher: address,
    //     sender_address: address,
    //     nonce: u64,
    //     txn_expiration_time: u64,
    // ) acquires NonceHistory {
    //     let nonce_history = borrow_global_mut<NonceHistory>(publisher);
    //     let nonce_key = NonceKey {
    //         sender_address,
    //         nonce,
    //     };
    //     let index = sip_hash_from_value(&nonce_key) % 200000;
    //     if (!table::contains(&nonce_history.nonce_table, index)) {
    //         let nonces = vector::empty();
    //         table::add(&mut nonce_history.nonce_table, index, Bucket {
    //             nonces: nonces,
    //         });
    //         return;
    //     };
    //     let bucket = table::borrow_mut(&mut nonce_history.nonce_table, index);
    // }

    // 68 microseconds per call, 432 gas/sec, 0.03 exe gas, 0 io gas for 3 million transactions
    // public(friend) entry fun check_and_insert_nonce(
    //     publisher: address,
    //     sender_address: address,
    //     nonce: u64,
    //     txn_expiration_time: u64,
    // ) {
    //     let nonce_key = NonceKey {
    //         sender_address,
    //         nonce,
    //     };
    //     sip_hash_from_value(&nonce_key) % 200000;
    // }

    // public(friend) entry fun check_and_insert_nonce(
    //     publisher: address,
    //     sender_address: address,
    //     nonce: u64,
    //     txn_expiration_time: u64,
    // ) acquires NonceHistory {
    //     let nonce_history = borrow_global_mut<NonceHistory>(publisher);
    //     let nonce_key = NonceKey {
    //         sender_address,
    //         nonce,
    //     };
    //     let index = sip_hash_from_value(&nonce_key) % 200000;
    //     if (!table::contains(&nonce_history.nonce_table, index)) {
    //         let nonces = vector::empty();
    //         table::add(&mut nonce_history.nonce_table, index, Bucket {
    //             nonces: nonces,
    //         });
    //         return;
    //     };
    //     let bucket = table::borrow_mut(&mut nonce_history.nonce_table, index);
    // }


                //////////////// Simple Map /////////////////////

    // public entry fun add_nonce_bucket(publisher: address) acquires NonceHistory {
    //     if (exists<NonceHistory>(publisher)) {
    //         let current_time = timestamp::now_seconds();
    //         let nonces = simple_map::new();
    //         let i = 0;
    //         while (i < 5) {
    //             let nonce_key = NonceKey {
    //                 sender_address: publisher,
    //                 nonce: current_time + i,
    //             };
    //             simple_map::add(&mut nonces, nonce_key, current_time + 100);
    //             i = i + 1;
    //         };
    //         let nonce_history = borrow_global_mut<NonceHistory>(publisher);
    //         if (!table::contains(&nonce_history.nonce_table, nonce_history.next_key)) {
    //             table::add(&mut nonce_history.nonce_table, nonce_history.next_key, Bucket {
    //                 nonces: nonces,
    //             });
    //         };
    //         nonce_history.next_key = nonce_history.next_key + 1;
    //     };
    // }

    // 1000 buckets, 2 entries per bucket. 50k entry function calls - 
    // 1000 buckets, 5 entries per bucket. 50k entry function calls - 684.1 us, 3315 gas/sec, 1.35 exe gas, 0.92 io gas
    // 1000 buckets, 10 entries per bucket. 50k entry function calls - 845.6 us, 2916 gas/sec, 1.54 exe gas, 0.92 io gas
    // 1000 buckets, 15 entries per bucket. 50k entry function calls - 1011.8 us, 2632 gas/sec, 1.74 exe gas, 0.92 io gas
    // 1000 buckets, 20 entries per bucket. 50k entry function calls - 1174.2 us, 2436 gas/sec, 1.94 exe gas, 0.92 io gas
    // public(friend) entry fun check_and_insert_nonce(
    //     publisher: address,
    //     sender_address: address,
    //     nonce: u64,
    //     txn_expiration_time: u64,
    // ) acquires NonceHistory {
    //     let nonce_history = borrow_global_mut<NonceHistory>(publisher);
    //     let nonce_key = NonceKey {
    //         sender_address,
    //         nonce,
    //     };
    //     let index = sip_hash_from_value(&nonce_key) % 1000;
    //     if (!table::contains(&nonce_history.nonce_table, index)) {
    //         let nonces = simple_map::new();
    //         simple_map::add(&mut nonces, nonce_key, txn_expiration_time);
    //         table::add(&mut nonce_history.nonce_table, index, Bucket {
    //             nonces: nonces,
    //         });
    //         return;
    //     };
    //     let bucket = table::borrow_mut(&mut nonce_history.nonce_table, index);
    //     if (simple_map::contains_key(&bucket.nonces, &nonce_key)) {
    //         return;
    //     };
    //     simple_map::add(&mut bucket.nonces, nonce_key, txn_expiration_time);
    // }



            //////////////// Ordered Map /////////////////////
    // public entry fun add_nonce_bucket(publisher: address) acquires NonceHistory {
    //     if (exists<NonceHistory>(publisher)) {
    //         let current_time = timestamp::now_seconds();
    //         let nonces = ordered_map::new();
    //         let i = 0;
    //         while (i < 20) {
    //             let nonce_key = NonceKey {
    //                 sender_address: publisher,
    //                 nonce: sip_hash_from_value(&(current_time + i)),
    //             };
    //             ordered_map::add(&mut nonces, nonce_key, current_time + 1500);
    //             i = i + 1;
    //         };
    //         let nonce_history = borrow_global_mut<NonceHistory>(publisher);
    //         if (!table::contains(&nonce_history.nonce_table, nonce_history.next_key)) {
    //             table::add(&mut nonce_history.nonce_table, nonce_history.next_key, Bucket {
    //                 nonces: nonces,
    //             });
    //         };
    //         nonce_history.next_key = nonce_history.next_key + 1;
    //     };
    // }

    // 1000 buckets, 2 entries per bucket. 50k entry function calls - 
    // 1000 buckets, 5 entries per bucket. 50k entry function calls - 495.8 us, 4227 gas/sec, 1.17 exe gas, 0.92 io gas
    // 1000 buckets, 10 entries per bucket. 50k entry function calls - 543.0 us, 3922 gas/sec, 1.21 exe gas, 0.92 io gas
    // 1000 buckets, 15 entries per bucket. 50k entry function calls - 602.0 us, 3594 gas/sec, 1.24 exe gas, 0.92 io gas
    // 1000 buckets, 20 entries per bucket. 50k entry function calls - 618.8 us, 3496 gas/sec, 1.24 exe gas, 0.92 io gas
    // public(friend) entry fun check_and_insert_nonce(
    //     publisher: address,
    //     sender_address: address,
    //     nonce: u64,
    //     txn_expiration_time: u64,
    // ) acquires NonceHistory {
    //     let nonce_history = borrow_global_mut<NonceHistory>(publisher);
    //     let nonce_key = NonceKey {
    //         sender_address,
    //         nonce,
    //     };
    //     let index = sip_hash_from_value(&nonce_key) % 1000;
    //     // if (!table::contains(&nonce_history.nonce_table, index)) {
    //     //     let nonces = ordered_map::new();
    //     //     ordered_map::add(&mut nonces, nonce_key, txn_expiration_time);
    //     //     table::add(&mut nonce_history.nonce_table, index, Bucket {
    //     //         nonces: nonces,
    //     //     });
    //     //     return;
    //     // };
    //     let bucket = table::borrow_mut(&mut nonce_history.nonce_table, index);
    //     ordered_map::upsert(&mut bucket.nonces, nonce_key, txn_expiration_time);
    // }

    // public(friend) entry fun check_and_insert_nonce(
    //     publisher: address,
    //     sender_address: address,
    //     nonce: u64,
    //     txn_expiration_time: u64,
    // ) acquires NonceHistory {
    //     let nonce_history = borrow_global_mut<NonceHistory>(publisher);
    //     let nonce_key = NonceKey {
    //         sender_address,
    //         nonce,
    //     };
    //     let index = sip_hash_from_value(&nonce_key) % 1000;
    //     if (!table::contains(&nonce_history.nonce_table, index)) {
    //         let nonces = ordered_map::new();
    //         ordered_map::add(&mut nonces, nonce_key, txn_expiration_time);
    //         table::add(&mut nonce_history.nonce_table, index, Bucket {
    //             nonces: nonces,
    //         });
    //         return;
    //     };
    //     let bucket = table::borrow_mut(&mut nonce_history.nonce_table, index);
    //     ordered_map::upsert(&mut bucket.nonces, nonce_key, txn_expiration_time);

    //     let current_time = timestamp::now_seconds();
    //     let len = ordered_map::length(&bucket.nonces);
    //     if (len > 15) {
    //         let new_bucket = Bucket {
    //             nonces: ordered_map::new(),
    //         };
    //         let i = 0;
    //         while (i < len) {
    //             let (cur_nonce_key, cur_txn_expiration_time) = ordered_map::pop_front(&mut bucket.nonces);
    //             if (current_time <= cur_txn_expiration_time) {
    //                 ordered_map::add(&mut new_bucket.nonces, cur_nonce_key, cur_txn_expiration_time);
    //             };
    //             i = i + 1;
    //         };
    //         *table::borrow_mut(&mut nonce_history.nonce_table, index) = new_bucket;
    //     } else {
    //         ordered_map::add(&mut bucket.nonces, nonce_key, txn_expiration_time);
    //     }
    // }



                //////////////// Big Ordered Map /////////////////////

    // public entry fun add_nonce_bucket(publisher: address) acquires NonceHistory {
    //     if (exists<NonceHistory>(publisher)) {
    //         let current_time = timestamp::now_seconds();
    //         let nonce_history = &mut borrow_global_mut<NonceHistory>(publisher);
    //         if (!table::contains(nonce_history.nonce_table, nonce_history.next_key)) {
    //             let nonces = big_ordered_map::new();
    //             let i = 0;
    //             while (i < 20) {
    //                 let nonce_key = NonceKey {
    //                     sender_address: publisher,
    //                     nonce: sip_hash_from_value(&(current_time + i)),
    //                 };
    //                 big_ordered_map::add(&mut nonces, nonce_key, current_time + 100);
    //                 i = i + 1;
    //             };
    //             table::add(nonce_history.nonce_table, nonce_history.next_key, Bucket {
    //                 nonces: nonces,
    //             });
    //         };
    //         *nonce_history.next_key = *nonce_history.next_key + 1;
    //     };
    // }

    // // 1000 buckets, 5 entries per bucket. 50k entry function calls - 958.5 us, 2374 gas/sec, 1.35 exe gas, 0.92 io gas
    // // 1000 buckets, 10 entries per bucket. 50k entry function calls - 1126.1 us, 2119 gas/sec, 1.47 exe gas, 0.92 io gas
    // // 1000 buckets, 15 entries per bucket. 50k entry function calls - 1156.1 us, 2064 gas/sec, 1.47 exe gas, 0.92 io gas
    // // 1000 buckets, 20 entries per bucket. 50k entry function calls - 1206.8 us, 2006 gas/sec, 1.50 exe gas, 0.92 io gas
    // public(friend) entry fun check_and_insert_nonce(
    //     publisher: address,
    //     sender_address: address,
    //     nonce: u64,
    //     txn_expiration_time: u64,
    // ) acquires NonceHistory {
    //     let nonce_history = &mut borrow_global_mut<NonceHistory>(publisher);
    //     let nonce_key = NonceKey {
    //         sender_address,
    //         nonce,
    //     };
    //     let index = sip_hash_from_value(&nonce_key) % 200;
    //     if (!table::contains(nonce_history.nonce_table, index)) {
    //         let nonces = big_ordered_map::new();
    //         big_ordered_map::add(&mut nonces, nonce_key, txn_expiration_time);
    //         table::add(nonce_history.nonce_table, index, Bucket {
    //             nonces: nonces,
    //         });
    //         return;
    //     };
    //     let bucket = &mut table::borrow_mut(nonce_history.nonce_table, index);
    //     big_ordered_map::upsert(bucket.nonces, nonce_key, txn_expiration_time);
    // }






                //////////////////// Ordered map 2 buckets ////////////////////

    struct Bucket has store {
        nonces: vector<OrderedMap<NonceKey, u64>>,
        last_cleared_times: vector<u64>,
    }

    struct NonceKey has copy, drop, store {
        sender_address: address,
        nonce: u64,
    }

    struct NonceHistory has key {
        // Key = sip_hash(NonceKey) % 200000
        // Value = Bucket { lowest_expiration_time: u64, nonces: vector<NonceEntry> }
        nonce_table: Table<u64, Bucket>,
        next_key: u64,
    }

    public entry fun add_nonce_bucket(publisher: address) acquires NonceHistory {
        if (exists<NonceHistory>(publisher)) {
            let current_time = timestamp::now_seconds();
            let nonce_history = borrow_global_mut<NonceHistory>(publisher);
            if (!table::contains(&nonce_history.nonce_table, nonce_history.next_key)) {
                let nonces = vector::empty();
                let last_cleared_times = vector::empty();
                vector::push_back(&mut nonces, ordered_map::new());
                vector::push_back(&mut nonces, ordered_map::new());
                vector::push_back(&mut last_cleared_times, current_time);
                vector::push_back(&mut last_cleared_times, current_time);
                let i = 0;
                while (i < 0) {
                    let nonce_key = NonceKey {
                        sender_address: publisher,
                        nonce: sip_hash_from_value(&(current_time + i)),
                    };
                    ordered_map::add(&mut nonces[0], nonce_key, current_time + 100);
                    let nonce_key = NonceKey {
                        sender_address: publisher,
                        nonce: sip_hash_from_value(&(current_time + i + 100)),
                    };
                    ordered_map::add(&mut nonces[1], nonce_key, current_time + 100);
                    i = i + 1;
                };
                table::add(&mut nonce_history.nonce_table, nonce_history.next_key, Bucket {
                    nonces: nonces,
                    last_cleared_times: last_cleared_times,
                });
            };
            nonce_history.next_key = nonce_history.next_key + 1;
        };
    }

    // // 100 buckets, 5 entries per bucket. 50k entry function calls - 808.8 us, 2828 gas/sec, 1.37 exe gas, 0.92 io gas
    // // 100 buckets, 10 entries per bucket. 50k entry function calls - 913.5 us, 2578 gas/sec, 1.43 exe gas, 0.92 io gas
    // // 100 buckets, 15 entries per bucket. 50k entry function calls - 1009.3 us, 2401 gas/sec, 1.50 exe gas, 0.92 io gas
    // // 100 buckets, 20 entries per bucket. 50k entry function calls - 1061.4 us, 2283 gas/sec, 1.50 exe gas, 0.92 io gas
    // public(friend) entry fun check_and_insert_nonce(
    //     publisher: address,
    //     sender_address: address,
    //     nonce: u64,
    //     txn_expiration_time: u64,
    // ) acquires NonceHistory {
    //     let nonce_history = borrow_global_mut<NonceHistory>(publisher);
    //     let nonce_key = NonceKey {
    //         sender_address,
    //         nonce,
    //     };
    //     let index = sip_hash_from_value(&nonce_key) % 100;
    //     let map_index = (txn_expiration_time/100) % 2;
    //     if (!table::contains(&nonce_history.nonce_table, index)) {
    //         let nonces = vector::empty();
    //         vector::push_back(&mut nonces, ordered_map::new());
    //         vector::push_back(&mut nonces, ordered_map::new());
    //         ordered_map::add(&mut nonces[map_index], nonce_key, txn_expiration_time);
    //         table::add(&mut nonce_history.nonce_table, index, Bucket {
    //             nonces: nonces,
    //         });
    //         return;
    //     };
    //     let bucket = table::borrow_mut(&mut nonce_history.nonce_table, index);
    //     if (!ordered_map::contains(&bucket.nonces[1-map_index], &nonce_key)) {
    //         ordered_map::upsert(&mut bucket.nonces[map_index], nonce_key, txn_expiration_time);
    //     }
    // }


    // // 100 buckets, 5 entries per bucket. 50k entry function calls - 1246.8 us, 2790 gas/sec, 1.64 exe gas, 1.84 io gas
    // // 100 buckets, 10 entries per bucket. 50k entry function calls - 
    // // 100 buckets, 15 entries per bucket. 50k entry function calls - 1178.3 us, 2401 gas/sec, 1.50 exe gas, 0.92 io gas
    // // 100 buckets, 20 entries per bucket. 50k entry function calls - 1218.0 us, 2804 gas/sec, 1.57 exe gas, 0.92 io gas

    public(friend) entry fun check_and_insert_nonce(
        publisher: address,
        sender_address: address,
        nonce: u64,
        txn_expiration_time: u64,
    ) acquires NonceHistory {
        let nonce_history = borrow_global_mut<NonceHistory>(publisher);
        let nonce_key = NonceKey {
            sender_address,
            nonce,
        };
        let index = sip_hash_from_value(&nonce_key) % 100;
        let map_index = (txn_expiration_time/100) % 2;
        if (!table::contains(&nonce_history.nonce_table, index)) {
            let nonces = vector::empty();
            let last_cleared_times = vector::empty();
            vector::push_back(&mut nonces, ordered_map::new());
            vector::push_back(&mut nonces, ordered_map::new());
            vector::push_back(&mut last_cleared_times, timestamp::now_seconds());
            vector::push_back(&mut last_cleared_times, timestamp::now_seconds());
            ordered_map::add(&mut nonces[map_index], nonce_key, txn_expiration_time);
            table::add(&mut nonce_history.nonce_table, index, Bucket {
                nonces: nonces,
                last_cleared_times: last_cleared_times,
            });
            return;
        };
        let bucket = table::borrow_mut(&mut nonce_history.nonce_table, index);
        if (!ordered_map::contains(&bucket.nonces[1-map_index], &nonce_key)) {
            ordered_map::upsert(&mut bucket.nonces[map_index], nonce_key, txn_expiration_time);
        };
        if (bucket.last_cleared_times[1-map_index] <= timestamp::now_seconds()) {
            bucket.nonces[1-map_index] = ordered_map::new();
            bucket.last_cleared_times[1-map_index] = timestamp::now_seconds();
        }
    }



                //////////////////// Big ordered map 2 buckets //////////////////////

    // struct Bucket has store {
    //     nonces: vector<BigOrderedMap<NonceKey, u64>>,
    // }

    // struct NonceKey has copy, drop, store {
    //     sender_address: address,
    //     nonce: u64,
    // }

    // struct NonceHistory has key {
    //     // Key = sip_hash(NonceKey) % 200000
    //     // Value = Bucket { lowest_expiration_time: u64, nonces: vector<NonceEntry> }
    //     nonce_table: Table<u64, Bucket>,
    //     next_key: u64,
    // }

    // public entry fun add_nonce_bucket(publisher: address) acquires NonceHistory {
    //     if (exists<NonceHistory>(publisher)) {
    //         let current_time = timestamp::now_seconds();
    //         let nonce_history = borrow_global_mut<NonceHistory>(publisher);
    //         if (!table::contains(&nonce_history.nonce_table, nonce_history.next_key)) {
    //             let nonces = vector::empty();
    //             vector::push_back(&mut nonces, big_ordered_map::new());
    //             vector::push_back(&mut nonces, big_ordered_map::new());
    //             let i = 0;
    //             while (i < 5) {
    //                 let nonce_key = NonceKey {
    //                     sender_address: publisher,
    //                     nonce: sip_hash_from_value(&(current_time + i)),
    //                 };
    //                 big_ordered_map::add(&mut nonces[0], nonce_key, current_time + 100);
    //                 let nonce_key = NonceKey {
    //                     sender_address: publisher,
    //                     nonce: sip_hash_from_value(&(current_time + i + 100)),
    //                 };
    //                 big_ordered_map::add(&mut nonces[1], nonce_key, current_time + 100);
    //                 i = i + 1;
    //             };
    //             table::add(&mut nonce_history.nonce_table, nonce_history.next_key, Bucket {
    //                 nonces: nonces,
    //             });
    //         };
    //         nonce_history.next_key = nonce_history.next_key + 1;
    //     };
    // }

    // // 100 buckets, 5 entries per bucket. 50k entry function calls - 1484.9 us, 2374 gas/sec, 1.35 exe gas, 0.92 io gas
    // // 100 buckets, 10 entries per bucket. 50k entry function calls - 1611.7 us, 1629 gas/sec, 1.70 exe gas, 0.92 io gas
    // // 100 buckets, 15 entries per bucket. 50k entry function calls - 1848.9 us, 1496 gas/sec, 1.84 exe gas, 0.92 io gas
    // // 100 buckets, 20 entries per bucket. 50k entry function calls - 1781.4 us, 1512 gas/sec, 1.77 exe gas, 0.92 io gas
    // public(friend) entry fun check_and_insert_nonce(
    //     publisher: address,
    //     sender_address: address,
    //     nonce: u64,
    //     txn_expiration_time: u64,
    // ) acquires NonceHistory {
    //     let nonce_history = borrow_global_mut<NonceHistory>(publisher);
    //     let nonce_key = NonceKey {
    //         sender_address,
    //         nonce,
    //     };
    //     let index = sip_hash_from_value(&nonce_key) % 100;
    //     let map_index = (txn_expiration_time/100) % 2;
    //     if (!table::contains(&nonce_history.nonce_table, index)) {
    //         let nonces = vector::empty();
    //         vector::push_back(&mut nonces, big_ordered_map::new());
    //         vector::push_back(&mut nonces, big_ordered_map::new());
    //         big_ordered_map::add(&mut nonces[map_index], nonce_key, txn_expiration_time);
    //         table::add(&mut nonce_history.nonce_table, index, Bucket {
    //             nonces: nonces,
    //         });
    //         return;
    //     };
    //     let bucket = table::borrow_mut(&mut nonce_history.nonce_table, index);
    //     if (!big_ordered_map::contains(&bucket.nonces[1-map_index], &nonce_key)) {
    //         big_ordered_map::upsert(&mut bucket.nonces[map_index], nonce_key, txn_expiration_time);
    //     }
    // }
}