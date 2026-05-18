// exclude_for: cvc5
// Tests vector<T> where T is a bitvector type, exercising Vec(bvN) generation
// in boogie_type(env, Type::Vector(et), bv_flag=true).
module 0x42::BvVectorElement {
    use std::vector;

    struct ByteVec has copy, drop {
        data: vector<u8>,
    }
    spec ByteVec {
        // Marks the data field (vector<u8>) as bv, producing Vec(bv8) in Boogie.
        pragma bv = b"0";
    }

    // Retrieve one byte from the bitvector-typed vector field.
    fun get_byte(b: &ByteVec, i: u64): u8 {
        *vector::borrow(&b.data, i)
    }
    spec get_byte {
        pragma bv_ret = b"0";
        aborts_if i >= len(b.data);
        ensures result == b.data[i];
    }

    // OR a mask into one byte of the bitvector-typed vector field.
    fun or_byte(b: &mut ByteVec, i: u64, mask: u8) {
        let byte = vector::borrow_mut(&mut b.data, i);
        *byte = *byte | mask;
    }
    spec or_byte {
        pragma bv = b"0,2";
        aborts_if i >= len(b.data);
        ensures b.data[i] == (old(b.data[i]) | mask);
        ensures forall j in 0..len(b.data): j != i ==> b.data[j] == old(b.data[j]);
    }

    // Two ByteVec values are element-wise equal iff their bv-typed data vectors match.
    fun same_data(a: &ByteVec, b: &ByteVec): bool {
        a.data == b.data
    }
    spec same_data {
        pragma bv = b"0,1";
        ensures result == (a.data == b.data);
    }
}
