// Type checking for `partial_of<f>(g)` / `captures_of<f>(g)` /
// `write_of<g, j>(args)`.
module 0x42::variant_observation_check {

    fun step(s: &mut u64, e: &u64) {
        *s = *s + *e;
    }
    spec step {
        ensures s == old(s) + e;
    }

    fun apply(f: |&u64| has copy + drop): |&u64| has copy + drop {
        f
    }
    spec apply {
        // Recognizer is bool; selector is typed by the captured param.
        ensures partial_of<f>(step) ==> captures_of<result>(step) == captures_of<f>(step);
        ensures result == f;
        // `write_of` with defaulted and explicit index.
        ensures write_of<step>(1, 2) == write_of<step, 0>(1, 2);
    }
}
