module 0x42::m {
    use std::vector;

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
        let x = vector::length(v);
        let len = vector::length(v);
        while (i < len) {
            if (f(vector::borrow(v, i))) {
                find = true;
                found_index = i;
                break
            };
            i = i + x;
        };
        spec {
            assert find ==> f(v[found_index]);
        };
        (find, found_index)
    }

    fun test(s: &mut S, issuer: vector<u8>) {
        let x = vector::length(&s.entries);
        let (_found, _index) = find(&s.entries, |obj| {
            spec {
                assume len(issuer) > 0;
            };
            obj.issuer == issuer && x > 0
        });
    }

}
