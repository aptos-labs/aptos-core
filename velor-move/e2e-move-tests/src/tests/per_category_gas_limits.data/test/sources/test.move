module 0xbeef::test {
    use std::signer::address_of;
    use std::vector;
    use velor_std::table::{Self, Table};

    struct Foo has key {}

    struct TableOfBytes has key {
        table: Table<u8, vector<u8>>,
    }

    entry fun run_move_to(s: signer) {
        move_to<Foo>(&s, Foo {});
    }

    entry fun run_exists() {
        exists<Foo>(@0x1);
    }

    entry fun init_table_of_bytes(s: &signer) {
        move_to(s, TableOfBytes { table: table::new() });
    }

    entry fun create_multiple(s: &signer, begin: u8, end: u8, base_size: u64) acquires TableOfBytes {
        // value size is at least 1 -- vector[] is of size 1
        assert!(base_size > 0, 0);
        let base_items = base_size - 1;
        let table_of_bytes = borrow_global_mut<TableOfBytes>(address_of(s));
        let t = &mut table_of_bytes.table;
        let k = begin;

        while (k < end) {
            // assert we are creating new items, return different error code for each k
            assert!(!table::contains(t, k), 1000 + (k as u64));

            // make a vector of size base_items + (k - begin), so items are always of different sizes
            let vec = vector[];
            let i = 0;
            while (i < base_items + ((k - begin) as u64)) {
                vector::push_back(&mut vec, k);
                i = i + 1;
            };

            // add to the table
            table::add(t, k, vec);

            k = k + 1;
        }
    }
}
