// Copyright © Aptos Foundation

// flag: --check-inconsistency
module 0x42::aborts_if_at_state_label {

    fun fee(nav: u64): u64 {
        nav / 2
    }
    spec fee {
        pragma opaque;
        ensures result == nav / 2;
        aborts_if false;
    }

    fun conv(amount: u64, denom: u64): u64 {
        amount / denom
    }
    spec conv {
        pragma opaque;
        ensures result == amount / denom;
        aborts_if denom == 0;
    }

    // Two opaque calls in sequence, so the contract has to speak about the
    // state between them. An `aborts_if` naming that state defines the label
    // `S1`, and the label definition is what the instrumenter needs. The
    // condition itself must not be assumed along with it: on the success path
    // it says the function aborted, which contradicts the arithmetic the body
    // just completed and makes every postcondition provable.
    fun caller(shares: u64, nav: u64): u64 {
        if (nav == 0) { return 0 };
        let f = fee(nav);
        let for_fee = shares * f / nav;
        let rest = shares - for_fee;
        conv(rest, nav)
    }
    spec caller {
        pragma opaque;
        ensures ({
            let a = {
                let b = ..S1 |~ result_of<fee>(nav);
                S1.. |~ result_of<conv>(shares - shares * b / nav, nav)
            };
            result == (if (nav == 0) 0 else a)
        });
        aborts_if S1 |~ (nav != 0 && {
            let a = ..S1 |~ result_of<fee>(nav);
            aborts_of<conv>(shares - shares * a / nav, nav)
        });
        aborts_if ({
            let a = ..S1 |~ result_of<fee>(nav);
            nav != 0 && shares < shares * a / nav
        });
        aborts_if ({
            let a = ..S1 |~ result_of<fee>(nav);
            nav != 0 && shares * a > MAX_U64
        });
    }
}
