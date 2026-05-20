// Test that a function with &mut struct param whose caller has a struct invariant
// does NOT produce invalid expressions like
// { let (_t0,_t1) = S1.. |~ result_of<f>(...); _t1 }.field
// which cause an ExpData::Invalid panic in the Boogie backend.
//
// The struct Pool has an invariant. f takes &mut Pool, making it a behavior
// predicate (first check in try_as_pure_spec_call fails). The WP engine
// then produces tuple-destructuring blocks: { let (_t0,_t1) = S1.. |~ result_of<f>(...); _t1 }
// which are then field-accessed (.data[x]) via the struct invariant injection.
module 0x42::mut_ref_invariant {
    struct Pool has copy, drop {
        value: u64,
        data: vector<u64>,
    }

    spec Pool {
        // A struct invariant that references the `data` field.
        // When WP processes a caller of f(&mut Pool), it must express the
        // post-Pool struct invariant in terms of the modified Pool, which is
        // { let (_t0,_t1) = S1.. |~ result_of<f>(...); _t1 }.
        // That produces { ... }.data[x] — a Select on a Block.
        invariant forall i in 0..len(data): data[i] >= 0;
    }

    // Takes &mut Pool → the first check in try_as_pure_spec_call fails
    // (no &mut params allowed). So this is always a behavior predicate.
    // result_of<f> has extended type (u64, Pool).
    fun f(self: &mut Pool, x: u64): u64 {
        self.value = self.value + x;
        self.value
    }
    spec f {
        pragma opaque = true;
        pragma inference = none;
        aborts_if self.value + x > MAX_U64;
        ensures self.value == old(self.value) + x;
        ensures result == self.value;
        ensures self.data == old(self.data);
    }

    // The WP for caller should express the post-state of self via
    // { let (_t0,_t1) = S1.. |~ result_of<f>(self, x); _t1 }.
    // The struct invariant on Pool then references .data[i] on this block.
    fun caller(self: &mut Pool, x: u64): u64 {
        f(self, x)
    }
}
