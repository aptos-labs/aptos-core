// flag: --verify-only=opaque_conditionless_strict_callee::caller
// Under `aborts_if_is_strict`, an opaque contract which states no abort
// condition states that the callee does not abort.
module 0x42::opaque_conditionless_strict_callee {
    spec module {
        pragma aborts_if_is_strict;
    }

    fun record(x: u64) {
        let _ = x;
    }
    spec record {
        pragma opaque;
    }

    fun caller(x: u64): u64 {
        record(x);
        x
    }
}
