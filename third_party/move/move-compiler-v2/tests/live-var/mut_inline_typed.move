module 0x42::m {

    use 0x1::vector;

    inline fun partition<Element>(
        v: &mut vector<Element>,
        pred: |&Element|bool
    ): u64 {
        let i = 0;
        let len = vector::length(v);
        while (i < len) {
            if (!pred(vector::borrow(v, i))) break;
            i = i + 1;
        };
        let p = i;
        i = i + 1;
        while (i < len) {
            if (pred(vector::borrow(v, i))) {
                vector::swap(v, p, i);
                p = p + 1;
            };
            i = i + 1;
        };
        p
    }

    fun foo(): u64 {
        let v = vector[1,2,3];
        let r = &mut v;
        partition(r, |e: &u64| *e > 1);
        *vector::borrow(r, 0)
    }

}
