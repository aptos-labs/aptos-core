// Behavior predicates — including `folds_of` — are only recognized starting
// at language version 2.4. Under earlier versions, `folds_of<f>(v, i)` is
// parsed as ordinary comparison expressions over the name `folds_of`.

module 0x42::M {

    inline fun apply(f: |u64|, v: vector<u64>, n: u64) {
        let i = 0;
        while (i < n) {
            f(i);
            i = i + 1;
        } spec {
            invariant folds_of<f>(v, i);
        };
    }
}
