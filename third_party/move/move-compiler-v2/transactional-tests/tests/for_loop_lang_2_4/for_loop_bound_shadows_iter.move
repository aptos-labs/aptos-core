//# run
script {
    // Before language version 2.5 the upper bound is evaluated with the iterator
    // in scope, so `for (i in 0..i)` reads the iterator (bound to the lower
    // bound, 0) and the loop runs zero times; this test pins the language version
    // to exercise that. (Since 2.5 the upper bound is evaluated outside the
    // iterator's scope and reads the enclosing `i`, so the same loop runs 5 times
    // -- see the sibling test under `control_flow/`.)
    fun main() {
        let i = 5;
        let count = 0;
        for (i in 0..i) {
            count = count + 1;
        };
        assert!(count == 0, 100); // zero iterations under <= 2.4 semantics
        assert!(i == 5, 101);     // enclosing `i` untouched
    }
}
