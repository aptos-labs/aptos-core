// flag: --check-inconsistency
//
// Checks that the verification context for `result_of` / `write_of_j` on
// `&mut`-bearing function values is satisfiable across the shapes the
// per-type evaluator handles (single `&mut`, multiple `&mut`, and
// multi-declared return + `&mut`).
module 0x42::result_of_mut_ref_soundness {

    // 1 declared + 1 `&mut`.
    fun apply_mut(f: |&mut u64| u64, x: &mut u64): u64 { f(x) }
    spec apply_mut {
        ensures result == result_of<f>(x);
        ensures ensures_of<f>(x, result);
    }

    // 1 declared + 2 `&mut`.
    fun apply_two_mut(f: |&mut u64, &mut u64| u64, p: &mut u64, q: &mut u64): u64 {
        f(p, q)
    }
    spec apply_two_mut {
        ensures result == result_of<f>(p, q);
        ensures ensures_of<f>(p, q, result);
    }

    // 2 declared + 1 `&mut`.
    fun apply_mut_multi(f: |&mut u64| (u64, u64), x: &mut u64): (u64, u64) { f(x) }
    spec apply_mut_multi {
        ensures (result_1, result_2) == result_of<f>(x);
    }
}
