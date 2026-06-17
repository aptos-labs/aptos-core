// A `&mut` reference to a vector element (`bucket`) is reborrowed when it is the
// receiver of an opaque call whose argument also reads the enclosing struct
// (`s.bucket_size`). The reborrow's lender and the reborrowed child reference are
// distinct mutable references that must stay distinct: coalescing them would drop
// the element's current value, leaving the bucket unconstrained and spuriously
// failing the data invariant. `touch` is an identity on the bucket, so the data
// invariant holds.
module 0x42::data_invariant_reborrow {
    use std::vector;

    struct Bucket has store, drop { v: vector<u64> }

    struct S has drop { buckets: vector<Bucket>, bucket_size: u64 }
    spec S {
        invariant forall i in 0..len(buckets):
            forall j in 0..len(buckets[i].v): buckets[i].v[j] > 0;
    }

    fun blen(self: &Bucket): u64 {
        vector::length(&self.v)
    }

    fun touch(self: &mut Bucket, _i: u64, _j: u64) {}
    spec touch {
        pragma opaque;
        aborts_if false;
        ensures self == old(self);
    }

    public fun reborrow_then_touch(s: &mut S, i: u64) {
        let bucket = vector::borrow_mut(&mut s.buckets, 0);
        let bucket_len = bucket.blen();
        bucket.touch(i % s.bucket_size, bucket_len - 1);
    }
}
