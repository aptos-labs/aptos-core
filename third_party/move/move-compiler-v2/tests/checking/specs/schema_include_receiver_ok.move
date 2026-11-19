module 0x42::M {
    struct S has copy, drop {
        value: u64
    }
    struct T has copy, drop {
        s: S,
        lim: u64
    }
    fun get_value(self: &S): u64 {
        self.value
    }

    spec schema Equal {
        r: num;
        v: num;
        ensures r == v;
    }
    spec schema AtMost {
        a: num;
        b: num;
        invariant a <= b;
    }
    spec schema EqualToValue {
        s: S;
        result: u64;
        include Equal { r: result, v: s.get_value() };
    }

    fun test_arg(s: &S): u64 {
        s.value
    }
    spec test_arg {
        include Equal { r: result, v: s.get_value() };
    }

    fun test_premise(s: &S): u64 {
        s.value
    }
    spec test_premise {
        include s.get_value() > 0 ==> Equal { r: result, v: s.value };
    }

    fun test_if(s: &S): u64 {
        s.value
    }
    spec test_if {
        include if (s.get_value() > 0) Equal { r: result, v: s.value } else Equal { r: result, v: 0 };
    }

    fun test_nested(s: &S): u64 {
        s.value
    }
    spec test_nested {
        include EqualToValue;
    }

    spec T {
        include AtMost { a: s.get_value(), b: lim };
    }
}
