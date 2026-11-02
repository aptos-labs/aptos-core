// flag: --verify-only=helper_bp::caller
// Inference scoped to one function leaves its callees without specification
// alone; the caller names them through behavioral predicates, which their
// bodies interpret.
// inference-reject-mutation: bump(s); => bump(s); bump(s);
module 0x42::helper_bp {
    struct S has drop {
        a: u64,
        b: u64,
    }

    fun bump(s: &mut S) {
        s.b = s.b + 1;
    }

    fun double(x: u64): u64 {
        x * 2
    }

    fun specified(x: u64): u64 {
        x
    }
    spec specified {
        ensures result == x;
        aborts_if false;
    }

    fun caller(s: &mut S, x: u64): u64 {
        bump(s);
        double(specified(x)) + 1
    }
    spec caller(s: &mut S, x: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == double(specified(x)) + 1;
        ensures [inferred] ensures_of<bump>(old(s), s);
        aborts_if [inferred] aborts_of<bump>(s);
        aborts_if [inferred] aborts_of<double>(specified(x));
        aborts_if [inferred] double(specified(x)) == MAX_U64;
    }


    fun unrelated(x: u64): u64 {
        double(x)
    }
}
/*
Verification: Succeeded.
Mutation: Rejected by postcondition.
*/
