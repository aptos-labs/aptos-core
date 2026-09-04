// flag: --check-inconsistency
// Repro: derived `ensures_of` for a closure puts the POST-state in the OLD slot
// when the closure returns a copy of the `&mut` parameter it just mutated.
//
// `returns_copy` FAILS with "post-condition does not hold"; `returns_const` passes.
// The two differ only in what the closure returns.
//
// Generated Boogie for the failing lambda:
//     $t2 := $Dereference($t1);      // entry snapshot  (correct old state)
//     call $t1 := dec($t1, $t0);     // the mutation
//     $t4 := $Dereference($t1);      // the `*x` return expression (post state)
//     assert ... $bp_ensures_of'dec'($t4, $t0, $t4);   // <-- $t4 in BOTH slots
// Since `dec` ensures `new.v == old.v - k`, that reduces to `k == 0`, which no
// real call satisfies. `$t2` is computed and never used.
//
// The passing control derives `$bp_ensures_of'dec'($t2, $t0, $Dereference($t1))`
// with the slots distinct.
//
// Found via aptos_market::single_order_book::decrease_order_size, whose closure
// is passed to big_ordered_map::modify_if_present_and_return (inline) and reaches
// the non-inline iter_modify, whose spec imports the closure contract.
module 0x42::bp_mut_return_copy_old_slot {
    struct S has copy, drop, store { v: u64 }

    fun dec(s: &mut S, k: u64) {
        s.v = s.v - k;
    }
    spec dec {
        aborts_if s.v < k;
        ensures s.v == old(s.v) - k;
    }

    /// Stands in for `big_ordered_map::iter_modify`: a non-inline HOF taking a
    /// real function value, whose contract imports the closure's contract.
    fun apply<R>(s: &mut S, f: |&mut S|R has drop): R {
        f(s)
    }
    spec apply {
        pragma opaque;
        pragma verify = false;
        aborts_if aborts_of<f>(s);
        ensures ensures_of<f>(old(s), result, s);
    }

    /// FAILS: the closure returns a copy of the parameter it just mutated.
    fun returns_copy(s: &mut S, k: u64): S {
        apply(s, |x| { dec(x, k); *x })
    }

    /// CONTROL, passes: identical except the returned value is not derived
    /// from the mutated parameter.
    fun returns_const(s: &mut S, k: u64): u64 {
        apply(s, |x| { dec(x, k); k })
    }
}
