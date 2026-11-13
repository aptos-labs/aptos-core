// A call through `&mut` followed by a write through the same reference: the
// postcondition must relate the call's result to the value the write starts
// from, not to the final value.
// inference-reject-mutation: b.amount = b.amount - n; => b.amount = b.amount - n - 1;
module 0x42::mut_call_then_write {
    struct Bucket has drop {
        amount: u64,
        cap: u64,
    }

    fun refill(b: &mut Bucket) {
        b.amount = b.cap;
    }
    spec refill {
        pragma opaque;
        ensures b == update_field(old(b), amount, old(b).cap);
        aborts_if false;
        ensures [inferred] b == update_field(old(b), amount, old(b).cap);
        aborts_if [inferred] false;
    }

    fun request(b: &mut Bucket, n: u64): bool {
        refill(b);
        if (b.amount >= n) {
            b.amount = b.amount - n;
            true
        } else {
            false
        }
    }
    spec request(b: &mut Bucket, n: u64): bool {
        pragma opaque = true;
        ensures [inferred] result == (exists x: Bucket: ensures_of<refill>(old(b), x) && x.amount >= n);
        ensures [inferred = sathard] exists x: Bucket: ensures_of<refill>(old(b), x) && (x.amount >= n ==> b == update_field(x, amount, x.amount - n));
        ensures [inferred = sathard] exists x: Bucket: ensures_of<refill>(old(b), x) && (x.amount < n ==> b == x);
        aborts_if [inferred] false;
    }

}
/*
Verification: Succeeded.
Mutation: Rejected by postcondition.
*/
