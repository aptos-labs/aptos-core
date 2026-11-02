module 0x42::loops {
    use std::vector;

    fun count_down(n: u64): u64 {
        let n = n;
        while (n > 0) {
            n = n - 1
        };
        n
    }
    spec count_down {
        ensures result == 0;
    }

    fun sum_to(n: u64): u64 {
        let i = 0;
        let total = 0;
        while (i < n) {
            total = total + i;
            i = i + 1;
        };
        total
    }

    fun labeled_exit(n: u64): u64 {
        let n = n;
        'outer: loop {
            loop {
                if (n < 1) break 'outer;
                n = n - 1;
                break
            }
        };
        n
    }

    fun first_even(v: &vector<u64>): u64 {
        let i = 0;
        let len = vector::length(v);
        while (i < len) {
            let x = *vector::borrow(v, i);
            if (x % 2 == 0) return x;
            i = i + 1;
        };
        0
    }

    fun build(n: u64): vector<u64> {
        let v = vector::empty<u64>();
        let i = 0;
        while (i < n) {
            vector::push_back(&mut v, i);
            i = i + 1
        };
        v
    }

    fun for_sum(n: u64): u64 {
        let total = 0;
        for (i in 0..n) {
            total = total + i
        };
        total
    }
}
