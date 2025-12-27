module 0x42::test {
    use std::vector;

    fun find<Element>(s: &vector<Element>, f: |&Element|bool has drop+copy): (bool, u64) {
        let find = false;
        let found_index = 0;
        let i = 0;
        let len = vector::length(s);
        while ({
            spec {
                invariant i <= len;
                invariant found_index == 0;
                invariant forall j in 0..i: !f(s[j]);
            };
            i < len
        }) {
            if (f(vector::borrow(s, i))) {
                find = true;
                found_index = i;
                break
            };
            i = i + 1;
        };
        (find, found_index)
    }
    spec find {
        pragma opaque;
        ensures result_1 <==> (exists i in range(s): f(s[i]));
        ensures result_1 ==> f(s[result_2]) && (forall i in 0..result_2: !f(s[i]));
    }

    fun pred(x: &u64): bool {
        *x > 1
    }

    fun call_find(): bool {
        let s = vector[1, 2, 3];
        let (found, _idx) = find(&s, pred);
        found
    }
    spec call_find {
        ensures result;
    }
}
