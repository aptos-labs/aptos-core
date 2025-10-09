module 0x8675309::M1 {


    struct R<phantom T> has key, store, copy, drop {
       x: u64
    }

    public fun extract<T>(r: &mut R<T>, y: u64): R<T> {
        let x = r.x - y;
        R {x}
    }

}

module 0x8675309::M {
    use 0x8675309::M1::R;
    use 0x8675309::M1::extract;

    struct R1<phantom T> has key {
       x: R<T>
    }


    fun f<T>(a: address): R<T> {
        let r = borrow_global_mut<R1<T>>(a);
        extract<T>(&mut r.x, 3)
    }

    public fun t0<T>(a: address): R<T> {
        assert!(false, 0);
        f(a)
    }

}
