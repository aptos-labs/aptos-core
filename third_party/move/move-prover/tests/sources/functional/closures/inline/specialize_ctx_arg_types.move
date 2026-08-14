// Free variables (and enclosing-function parameters) of unified lambda
// material become typed context parameters of the shared specialization, so
// spec equivalence requires them to agree in reference-stripped type, not
// just in symbol/index. Without the type requirement, the same-shaped
// lambdas below — whose captures share the parameter index but differ in
// type — would share one specialization: `cc_flag`'s boolean context
// parameters would receive `dd_flag`'s u64 arguments (a Boogie type error),
// and `bb_count`'s u128 capture would flow through a u64-typed parameter.
module 0x42::specialize_ctx_arg_types {

    /// Recursive fold over the prefix `v[0..end]`.
    spec fun spec_fold(f: |u64, u64| u64, v: vector<u64>, init: u64, end: u64): u64 {
        if (end == 0) init
        else result_of<f>(spec_fold(f, v, init, end - 1), v[end - 1])
    }

    // ===== same symbol/index, u64 vs u128 =====

    fun aa_count(v: vector<u64>, k: u64): u64 {
        let _ = v;
        let _ = k;
        0
    }
    spec aa_count {
        ensures result == spec_fold(|acc, e| if (e >= k) acc else acc + e, v, 0, 0);
    }

    fun bb_count(v: vector<u64>, k: u128): u64 {
        let _ = v;
        let _ = k;
        0
    }
    spec bb_count {
        ensures result == spec_fold(|acc, e| if (e >= k) acc else acc + e, v, 0, 0);
    }

    // ===== same symbols/indices, bool vs u64 (different Boogie types) =====

    fun cc_flag(v: vector<u64>, p: bool, q: bool): u64 {
        let _ = v;
        let _ = p;
        let _ = q;
        0
    }
    spec cc_flag {
        ensures result == spec_fold(|acc, e| if (p == q) acc + e else acc, v, 0, 0);
    }

    fun dd_flag(v: vector<u64>, p: u64, q: u64): u64 {
        let _ = v;
        let _ = p;
        let _ = q;
        0
    }
    spec dd_flag {
        ensures result == spec_fold(|acc, e| if (p == q) acc + e else acc, v, 0, 0);
    }
}
