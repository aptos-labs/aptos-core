module 0x42::m {
    use std::vector;

    struct Option<Element> has copy, drop, store {
        vec: vector<Element>
    }

    public fun is_none<Element>(t: &Option<Element>): bool {
        vector::is_empty(&t.vec)
    }

    public fun destroy_none<Element>(t: Option<Element>) {
        assert!(is_none(&t), 0x40000);
        let Option { vec } = t;
        vector::destroy_empty(vec)
    }

    struct E<Key> has copy, drop, store {
        key: Key,
    }

    inline fun h<Key: store + drop>(x: E<Key>, v: |Key| E<Key>): E<Key> {
        let E { key } = x;
        v(key)
    }

    public inline fun g<Key: store + drop>(x: E<Key>, v: |E<Key>|) {
        v(x)
    }

    public fun foo<Key: store + drop>(
        data: E<Key>, v: &mut Key) {
        let f = h(data, |e: Key| {
            E {key: e}
        });
        g(f, |e: E<Key>| {
            let (E { key }, _x) = (e, 3);
            *v = key;
        });
    }

}
