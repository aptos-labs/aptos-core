spec std::error {

    spec aborted(r: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == canonical(7, r);
        aborts_if [inferred] aborts_of<canonical>(7, r);
    }

    spec already_exists(r: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == canonical(8, r);
        aborts_if [inferred] aborts_of<canonical>(8, r);
    }

    spec canonical(category: u64, reason: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == (category << 16) + reason;
        aborts_if [inferred](category << 16) + reason > MAX_U64;
    }

    spec internal(r: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == canonical(11, r);
        aborts_if [inferred] aborts_of<canonical>(11, r);
    }

    spec invalid_argument(r: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == canonical(1, r);
        aborts_if [inferred] aborts_of<canonical>(1, r);
    }

    spec invalid_state(r: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == canonical(3, r);
        aborts_if [inferred] aborts_of<canonical>(3, r);
    }

    spec not_found(r: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == canonical(6, r);
        aborts_if [inferred] aborts_of<canonical>(6, r);
    }

    spec not_implemented(r: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == canonical(12, r);
        aborts_if [inferred] aborts_of<canonical>(12, r);
    }

    spec out_of_range(r: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == canonical(2, r);
        aborts_if [inferred] aborts_of<canonical>(2, r);
    }

    spec permission_denied(r: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == canonical(5, r);
        aborts_if [inferred] aborts_of<canonical>(5, r);
    }

    spec resource_exhausted(r: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == canonical(9, r);
        aborts_if [inferred] aborts_of<canonical>(9, r);
    }

    spec unauthenticated(r: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == canonical(4, r);
        aborts_if [inferred] aborts_of<canonical>(4, r);
    }

    spec unavailable(r: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == canonical(13, r);
        aborts_if [inferred] aborts_of<canonical>(13, r);
    }
}
