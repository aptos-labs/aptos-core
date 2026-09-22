// inference-reject-mutation: // MUTATION_POINT => return 0;
module 0x42::ghost_assumption {
    spec module {
        global counter: u64;
    }

    fun helper(): u64 { 7 }
    spec helper {
        pragma opaque;
        aborts_if false;
        ensures result == 7;
        ensures counter == counter;
        ensures [inferred] result == 7;
        aborts_if [inferred] false;
    }

    // The callee's ghost state must not erase normal or abort behavior.
    fun caller(x: u64): u64 {
        // MUTATION_POINT
        helper() + x
    }
    spec caller(x: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == helper() + x;
        aborts_if [inferred] helper() + x > MAX_U64;
    }

}
/*
Verification: Succeeded.
Mutation: Rejected by postcondition.
*/
