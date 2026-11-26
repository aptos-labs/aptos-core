// Two variants of one enum with the vector field in different positions (first in `Left`,
// last in `Right`), kept in one vector across two collections, so each collection must
// find each element's vector by its variant. Decoy enums with different contents are
// allocated between the collections, and the originals are read only after the second.
// The golden pins the lowering and frame layout.

// RUN: publish --print(micro-ops,frame-layout)
module 0x42::variant_maps {
    use std::vector;

    enum Pair has drop { Left { bytes: vector<u8>, n: u64 }, Right { n: u64, bytes: vector<u8> } }

    fun bytes_of(first: u8, len: u64): vector<u8> {
        let bytes = vector::empty<u8>();
        let index = 0;
        while (index < len) { vector::push_back(&mut bytes, first + (index as u8)); index = index + 1; };
        bytes
    }

    fun weight(pair: Pair): u64 {
        match (pair) {
            Pair::Left { bytes, n: num } => num * 1000 + (*vector::borrow(&bytes, 0) as u64),
            Pair::Right { n: num, bytes } => num * 1000 + (*vector::borrow(&bytes, 0) as u64),
        }
    }

    public fun both(): (u64, vector<u8>, u64, vector<u8>, u64) {
        let pairs = vector[Pair::Left { bytes: bytes_of(0x78, 1), n: 3 }, Pair::Right { n: 4, bytes: bytes_of(0x79, 2) }];
        0x0::test_utils::force_gc();
        let decoys = vector[Pair::Right { n: 98, bytes: bytes_of(0x41, 2) }, Pair::Left { bytes: bytes_of(0x51, 1), n: 99 }];
        0x0::test_utils::force_gc();
        let Pair::Right { n: n2, bytes: b2 } = vector::pop_back(&mut pairs);
        let Pair::Left { bytes: b1, n: n1 } = vector::pop_back(&mut pairs);
        let decoy_weight = weight(vector::pop_back(&mut decoys)) + weight(vector::pop_back(&mut decoys));
        (n1, b1, n2, b2, decoy_weight)
    }
}

// RUN: execute 0x42::variant_maps::both
// CHECK: results: 3, 0x78, 4, 0x797a, 197146
// CHECK-GC-COUNT: 2
