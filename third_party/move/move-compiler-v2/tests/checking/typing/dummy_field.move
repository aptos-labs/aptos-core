module 0x42::test {

    struct R has store, key, drop {
        dummy_field: bool,
    }

    public entry fun test(addr: address) acquires R {
        let R { dummy_field: _dummy_field  } = move_from<R>(addr);
    }

    fun test2(): bool {
        let r = R {
            dummy_field: true
        };
        r.dummy_field
    }

    struct T has store, key, drop {
    }

    public entry fun test3(addr: address) acquires T {
        let T {  } = move_from<T>(addr);
    }

    public entry fun test4(s: &signer) {
        let r = T {};
        move_to<T>(s, r);
    }

}
