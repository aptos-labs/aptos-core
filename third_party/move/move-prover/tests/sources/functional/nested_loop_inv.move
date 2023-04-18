module 0x42::nested_loop {
    use std::vector;
    public fun assert_no_duplicate(v: &vector<u64>) {
        let i = 0;
        let n = vector::length(v);
        if (n == 0) {
            return
        };
        while({
            spec {
                invariant i <= n-1;
                invariant forall x: u64, y: u64 where x < i && x < y && y < n: v[x] != v[y];
            };
            i < n-1
            }) {
            let j = i+1;
            while({
                spec {
                    invariant i <= n-1;
                    invariant j <= n;
                    invariant i < j;
                    invariant forall y: u64 where i < y && y < j: v[i] != v[y];
                };
                j < n
            }) {
                let v_i = *vector::borrow(v, i);
                let v_j = *vector::borrow(v, j);
                assert!(v_i != v_j, 0);
                j = j + 1;
            };
            i = i + 1;
        }
    }
    spec assert_no_duplicate {
        aborts_if exists i: u64, j: u64: i < len(v) && j < len(v) && i != j && v[i] == v[j];
    }
}
