// Twins are per-instance: a struct-valued sibling instantiation of the same
// map type must not suppress the bv twin demanded by a bitwise-classified
// numeric-valued instantiation.
module 0x42::VerifyBitwiseTableMixedInstances {
    use extensions::table::{Self, Table};
    use extensions::table::spec_get;

    struct Item has copy, drop, store {
        a: u64,
    }

    fun structs(): Table<u8, Item> {
        let t = table::new<u8, Item>();
        table::add(&mut t, 1, Item { a: 2 });
        t
    }
    spec structs {
        ensures spec_get(result, 1).a == 2;
    }

    fun packed(): Table<u8, u64> {
        let t = table::new<u8, u64>();
        table::add(&mut t, 1, 3 & 7);
        t
    }
    spec packed {
        pragma bv_ret = b"0";
        ensures spec_get(result, 1) == (3 as u64);
    }

    // Nested unsigned value types have a bv rendering too: the twin supply
    // must cover them, or a Bitwise-classified instantiation selects an
    // unemitted `Table int (Vec bvN)` representation.
    fun packed_vec(x: u8): Table<u8, vector<u8>> {
        let t = table::new<u8, vector<u8>>();
        table::add(&mut t, 1, vector[x & 1]);
        t
    }
    spec packed_vec {
        aborts_if false;
        ensures spec_get(result, 1) == vector[x & 1];
    }
}
