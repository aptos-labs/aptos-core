module poc::fv_any {
    use std::any;
    struct A {}

    public fun make_potato(): A {
        A {}
    }

    public fun destroy_potato(x: A) {
        A {} = x;
    }

    public fun zzz(): || {
        let potato = make_potato();
        || destroy_potato(potato)
    }

    public entry fun win() {
        let x: || (||) has drop + store = zzz;

        let a = 0x1::any::pack(x);

        let b = 0x1::any::unpack<|| (|| has drop + store)>(a);
        b();
    }
}
