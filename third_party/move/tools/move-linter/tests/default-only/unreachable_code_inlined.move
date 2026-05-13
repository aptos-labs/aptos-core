// Dead code that originates from an inlined call should NOT warn — the
// synthesized trailing `Ret` is an inlining artifact.
//
// Genuinely dead code inside an inline function body is also not flagged at the
// call site: this is a known false negative.
module 0xc0ffee::m {
    inline fun terminator() {
        abort 0
    }

    public fun caller() {
        terminator();
    }

    inline fun dead_in_body(): u64 {
        abort 0;
        42  // genuinely dead, but won't warn because it's inlined at the call site
    }

    public fun caller2(): u64 {
        dead_in_body()
    }
}
