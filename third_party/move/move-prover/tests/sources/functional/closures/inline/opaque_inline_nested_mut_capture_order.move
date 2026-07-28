// Regression test: a variable which the outer lambda reads BEFORE a nested
// lambda mutates it must still be captured `&mut` by the outer lambda (the
// `modified` flag recorded while lifting the nested lambda survives the
// context restore), so the mutation reaches the caller's variable.
module 0x42::opaque_inline_nested_mut_capture_order {

    inline fun call_once(f: |u64|) {
        f(1)
    }
    spec call_once {
        pragma opaque;
        ensures ensures_of<f>(1);
    }

    inline fun call_outer(g: ||) {
        g()
    }
    spec call_outer {
        pragma opaque;
        ensures ensures_of<g>();
    }

    fun read_before_nested_mutation(): u64 {
        let x = 0;
        let r = 0;
        call_outer(|| {
            let a = x; // read of `x` before the nested mutating lambda
            call_once(|i| x = x + i spec { ensures x == old(x) + i; });
            r = a;
        } spec { ensures x == old(x) + 1 && r == old(x); });
        x + r
    }
    spec read_before_nested_mutation {
        ensures result == 1;
    }

    fun read_before_nested_mutation_incorrect(): u64 {
        let x = 0;
        let r = 0;
        call_outer(|| {
            let a = x;
            call_once(|i| x = x + i spec { ensures x == old(x) + i; });
            r = a;
        } spec { ensures x == old(x) + 1 && r == old(x); });
        x + r
    }
    spec read_before_nested_mutation_incorrect {
        ensures result == 0; // error: the nested mutation of `x` reaches the caller
    }
}
