//# run
script {
    // This test compiles at the latest language version, where the upper bound
    // is evaluated OUTSIDE the iterator's scope: in `for (i in 0..i)` it reads
    // the enclosing `i`, not the freshly-bound iterator. With the enclosing
    // `i = 5` the loop runs 5 times (i = 0,1,2,3,4) and leaves `i` unchanged.
    //
    // Before language version 2.5 the upper bound was evaluated with the iterator
    // in scope, so `0..i` read the iterator (= 0) and the loop ran 0 times -- see
    // the sibling test under `for_loop_lang_2_4/`.
    fun main() {
        let i = 5;
        let count = 0;
        let sum = 0;
        for (i in 0..i) {
            count = count + 1;
            sum = sum + i;
        };
        assert!(count == 5, 100);
        assert!(sum == 0 + 1 + 2 + 3 + 4, 101); // 10
        assert!(i == 5, 102);                    // enclosing `i` untouched
    }
}
