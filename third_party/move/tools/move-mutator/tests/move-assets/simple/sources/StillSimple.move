module TestAccount::StillSimple {
    fun sample1(x: u128, y: u128) {
        let _sum_r = x + y;

	    if ((x + y) < 0) abort 1;
    }

    inline fun apply(v: u64, predicate: |u64| bool): bool {
        predicate(v)
    }

    fun sample2(a1: u64, a2: u64) {
        let _lamb = apply(0, |v| v != a1 + a2);
    }

    public fun sample3(n: u64, e: u64): u64 {
        if (e == 0) {
            1
        } else {
            n * sample3(n, e - 1)
        }
    }

    spec sample3 {
        pragma opaque;
    }

    fun sample4(x: u128, y: u128) {
        loop {
            if (x > y) {
                y = y + 1;
                continue
            };
            if (y > x) {
                x = x + 1;
                continue
            };
            break
        };
    }

    fun sample5(x: u128, y: u128): u128 {
        (x - 1 as u128) + (y - 1 as u128)
    }

    fun sample6(x: u128, y: u128): u128 {
        return (x + y - x*(y + 2)*x/y)
    }
}
