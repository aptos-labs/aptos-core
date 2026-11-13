// A callee whose contract determines each of several results as a value
// publishes those values to its callers, as a single-result callee's does:
// the caller's inferred conditions read the callee's arguments directly
// rather than through `result_of` projections, which the solver could only
// relate back to the call's results through the carrier axiom.
module 0x42::functional_tuple_result {
    struct Info has copy, drop { a: u64, b: u64 }

    // A loop makes `helper`, and so `split`, impure under spec semantics:
    // `split` cannot become a pure spec call and is reached as a callee.
    fun helper(n: u64): u64 {
        let i = 0;
        while ({
            spec { invariant i <= n; };
            i < n
        }) {
            i = i + 1;
        };
        i
    }
    spec helper {
        pragma opaque;
        pragma inference = none;
        aborts_if false;
        ensures result == n;
    }

    fun split(info: Info): (u64, u64) {
        (helper(info.a), helper(info.b))
    }
    spec split {
        pragma opaque;
        pragma inference = none;
        aborts_if false;
        ensures result_1 == info.a;
        ensures result_2 == info.b;
    }

    // Only one of the two results is determined: `result_of` stays.
    fun split_partial(info: Info): (u64, u64) {
        (helper(info.a), helper(info.b))
    }
    spec split_partial {
        pragma opaque;
        pragma inference = none;
        aborts_if false;
        ensures result_1 == info.a;
    }

    fun sum(info: Info): u64 {
        let (a, b) = split(info);
        a + b
    }
    spec sum(info: Info): u64 {
        pragma opaque = true;
        ensures [inferred] result == info.a + info.b;
        aborts_if [inferred] info.a + info.b > MAX_U64;
    }


    fun sum_partial(info: Info): u64 {
        let (a, b) = split_partial(info);
        a + b
    }
    spec sum_partial(info: Info): u64 {
        pragma opaque = true;
        ensures [inferred] ({
            let (a_0,a_1) = result_of<split_partial>(info);
            result == a_0 + a_1
        });
        aborts_if [inferred] ({
            let (a_0,a_1) = result_of<split_partial>(info);
            a_0 + a_1 > MAX_U64
        });
    }

}
/*
Verification: Succeeded.
*/
