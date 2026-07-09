// RUN: publish
module 0x42::enums_in_vector {
    use std::vector;

    enum Item has drop {
        Empty,
        Num { n: u64 },
    }

    // A vector of enums built with push_back (VecPack with elements isn't
    // lowered yet). Borrow each element and match on the reference.
    fun sum_items(a: u64, b: u64): u64 {
        let v = vector::empty<Item>();
        vector::push_back(&mut v, Item::Num { n: a });
        vector::push_back(&mut v, Item::Empty);
        vector::push_back(&mut v, Item::Num { n: b });
        let total = 0;
        let i = 0;
        let len = vector::length(&v);
        while (i < len) {
            let item = vector::borrow(&v, i);
            match (item) {
                Item::Empty => {},
                Item::Num { n } => total = total + *n,
            };
            i = i + 1;
        };
        total
    }
}

// RUN: execute 0x42::enums_in_vector::sum_items --args 30, 12
// CHECK: results: 42
