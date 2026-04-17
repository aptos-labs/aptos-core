// Dead code that originates from an inlined call should NOT warn — the
// synthesized trailing `Ret` is an inlining artifact.
module 0xc0ffee::m {
    inline fun terminator() {
        abort 0
    }

    public fun caller() {
        terminator();
    }
}
