module 0x42::test {

    struct R has store, key, drop {
        dummy_field: bool,
    }

    public entry fun test(addr: address) acquires R {
        let R {  } = move_from<R>(addr);
    }

    public entry fun test2(s: &signer) {
        let r = R {};
        move_to<R>(s, r);
    }

    struct T has store, key, drop {
    }

    public entry fun test3(addr: address) acquires T {
        let T { dummy_field } = move_from<T>(addr);
    }

    fun test4(): bool {
        let t = T {
        };
        t.dummy_field
    }

    struct G has store, key, drop {
        dummy_field_1: bool,
    }

    public entry fun test5(addr: address) acquires G {
        let G {  } = move_from<G>(addr);
    }

    public entry fun test6(s: &signer) {
        let r = G {};
        move_to<G>(s, r);
    }

    struct A has key, drop {
    }

    public fun test7(input: address): bool acquires A {
        let a = move_from(input);
        let x = a.dummy_field;
        let A {} = a;
        x
    }

}
