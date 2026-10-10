// A semantic quantifier in the inferred contract must not be mistaken for
// residual loop-havoc state when the supplied invariant summarizes the loop.
module 0x42::semantic_quantifier_loop {

    fun contains(values: &vector<u64>, needle: u64): bool {
        let i = 0;
        while (i < values.length()) {
            if (values[i] == needle) {
                return true
            };
            i += 1;
        } spec {
            invariant i <= len(values);
            invariant forall j: num: 0 <= j && j < i ==> values[j] != needle;
        };
        false
    }
}
