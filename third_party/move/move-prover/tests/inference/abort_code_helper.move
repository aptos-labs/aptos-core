// A helper used only to construct an abort code cannot contribute a normal
// postcondition or make an unconditional abort continuation incomplete.
module 0x42::abort_code_helper {
    use std::error;

    fun divide(a: u64, b: u64): u64 {
        assert!(b != 0, error::invalid_argument(4));
        a / b
    }

    fun always_aborts() {
        abort error::invalid_argument(4)
    }

    // Shape of the VS-shares-002 failure, including the inlined helper and cast.
    inline fun mul_div(a: u64, b: u64, c: u64): u64 {
        assert!(c != 0, error::invalid_argument(4));
        (((a as u128) * (b as u128) / (c as u128)) as u64)
    }

    fun shares(shares: u64, nav: u64, total: u64): u64 {
        assert!(total >= shares, 1);
        mul_div(shares, nav, total)
    }

    // An incomplete callee on a returning path must still be reported.
    fun unknown(x: u64): u64 { x + 1 }
    spec unknown {
        pragma opaque;
        pragma verify = false;
        pragma inference = none;
        pragma aborts_if_is_partial;
        ensures result == x + 1;
    }

    fun returning_call(x: u64): u64 { unknown(x) }

    fun guarded_call(x: u64, call: bool): u64 {
        if (call) { unknown(x) } else { 0 }
    }

    // There is no normal return, but the abort continuation is conditional:
    // an unknown callee before it must not be mistaken for an abort-code helper.
    fun conditional_abort(x: u64) {
        let value = unknown(x);
        loop {
            spec { invariant true; };
            if (value != 0) { break };
        };
        abort 1
    }
}
