/// Regression test: the transaction_context counter model must be consistent.
/// With `spec_counter_issued` returning u128, its strict-monotonicity axiom has no
/// model, and the loop below proves a false postcondition by induction over it.
module counter_model::counter_model {
    use aptos_framework::transaction_context;

    /// 2^128
    const N: u256 = 340282366920938463463374607431768211456;

    /// Never calls the counter; the invariant follows from the axiom by induction.
    fun walk(): u64 {
        let i: u256 = 0;
        while ({
            spec {
                invariant i <= N;
                invariant transaction_context::spec_counter_issued(i)
                    >= transaction_context::spec_counter_issued(0) + i;
            };
            i < N
        }) {
            i = i + 1;
        };
        0
    }

    // False: must not verify.
    spec walk {
        ensures result == 1;
    }

    /// Control: the same false claim without the loop.
    fun zero(): u64 {
        0
    }

    spec zero {
        ensures result == 1;
    }
}
