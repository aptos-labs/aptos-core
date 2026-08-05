// Restrictions on modifying captured variables in lambdas passed to retained
// inline-opaque functions: converted variables cannot appear in tuple
// assignments. (A mutating lambda for a `copy`-requiring function parameter is
// rejected by the closure checker; see the prover test suite.)
module 0x42::retained_mut_capture_err {

    inline fun call_once(f: |u64|) {
        f(1)
    }
    spec call_once {
        pragma opaque;
        ensures ensures_of<f>(1);
    }

    /// Rejected: converted variable in a tuple assignment.
    fun tuple_caller(): u64 {
        let x = 0;
        let y = 0;
        call_once(|i| (x, y) = (i, i));
        x + y
    }
}
