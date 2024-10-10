module 0x42::m {
    use std::vector;
    use std::option;
    use std::option::Option;

    struct J has copy, drop, store {
        variant: u64,
    }

    struct T has copy, drop, store {
        issuer: vector<u8>,
        version: u64,
    }

    struct S has copy, drop, store {
        entries: vector<T>,
    }

    public inline fun find<Element>(v: &vector<Element>, f: |&Element|bool): (bool, u64) {
        let find = false;
        let found_index = 0;
        let i = 0;
        let len = vector::length(v);
        while (i < len) {
            if (f(vector::borrow(v, i))) {
                find = true;
                found_index = i;
                break
            };
            i = i + 1;
        };
        spec {
            assert find ==> f(v[found_index]);
        };
        (find, found_index)
    }

    fun test(s: &mut S, issuer: vector<u8>): Option<T> {
        let (found, index) = find(&s.entries, |obj| {
            let set: &T = obj;
            set.issuer == issuer
        });

        let ret = if (found) {
            option::some(vector::remove(&mut s.entries, index))
        } else {
            option::none()
        };

        ret
    }

    fun test1() {
        let t0 = T {
            issuer: vector[1],
            version: 1
        };
        let t1 = T {
            issuer: vector[2],
            version: 0
        };
        let s = S {
            entries: vector[t0, t1]
        };
        let _opt_t = test(&mut s, vector[0]);

    }

}
