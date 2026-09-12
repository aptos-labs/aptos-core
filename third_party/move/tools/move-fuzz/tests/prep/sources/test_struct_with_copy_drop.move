module test::struct_with_copy_drop {
    struct S {
        a: address
    }
    has copy, drop;

    public fun create(addr: address): S {
        S { a: addr }
    }

    public fun get_address_owned(s: S): address {
        s.a
    }

    public fun get_address_immref(s: &S): address {
        s.a
    }

    public fun get_address_mutref(s: &mut S): address {
        s.a
    }
}
