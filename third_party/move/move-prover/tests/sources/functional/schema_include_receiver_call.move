// Receiver-style calls inside schema inclusion expressions: arguments, `==>` premises and
// `if` conditions, in function specs, struct specs, nested schemas and `apply`.
module 0x42::schema_include_receiver_call {
    struct S has copy, drop { f: u8, inner: In }
    struct In has copy, drop { x: u64 }
    struct W<X> has drop { n: u64, t: X }
    struct Bounded has copy, drop { s: S, lim: u64 }

    fun get_f(self: &S): u8 { self.f }
    fun get_inner(self: &S): In { self.inner }
    fun inner_ref(self: &S): &In { &self.inner }
    fun get_x(self: &In): u64 { self.x }
    fun n_of<X>(self: &W<X>): u64 { self.n }

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

    spec schema EqualToF {
        s: S;
        result: u64;
        include Equal { r: result, v: s.get_f() };
    }

    fun f_of(s: &S): u64 { (s.f as u64) }
    spec f_of {
        include Equal { r: result, v: s.get_f() };
    }

    fun f_of_incorrect(s: &S): u64 { (s.f as u64) }
    spec f_of_incorrect {
        include Equal { r: result, v: s.get_f() + 1 };
    }

    fun f_or_zero(s: &S): u64 { if (s.f > 0) (s.f as u64) else 0 }
    spec f_or_zero {
        include s.get_f() > 0 ==> Equal { r: result, v: s.f };
        include s.get_f() == 0 ==> Equal { r: result, v: 0 };
    }

    fun f_or_zero_if(s: &S): u64 { if (s.f > 0) (s.f as u64) else 0 }
    spec f_or_zero_if {
        include if (s.get_f() > 0) Equal { r: result, v: s.get_f() } else Equal { r: result, v: 0 };
    }

    fun f_or_zero_incorrect(s: &S): u64 { if (s.f > 0) (s.f as u64) else 0 }
    spec f_or_zero_incorrect {
        include s.get_f() > 0 ==> Equal { r: result, v: s.f + 1 };
    }

    fun f_or_zero_if_incorrect(s: &S): u64 { if (s.f > 0) (s.f as u64) else 0 }
    spec f_or_zero_if_incorrect {
        include if (s.get_f() > 0) Equal { r: result, v: s.get_f() } else Equal { r: result, v: 1 };
    }

    fun f_of_nested(s: &S): u64 { (s.f as u64) }
    spec f_of_nested {
        include EqualToF;
    }

    fun x_of(s: &S): u64 { s.inner.x }
    spec x_of {
        include Equal { r: result, v: s.get_inner().x };
    }

    fun x_of_chained(s: &S): u64 { s.inner.x }
    spec x_of_chained {
        include Equal { r: result, v: s.inner_ref().get_x() };
    }

    fun f_of_lambda(s: &S): u64 { (s.f as u64) }
    spec f_of_lambda {
        include Equal { r: result, v: ((|t: S| t.get_f())(s) as u64) };
    }

    fun take_f(s: &mut S): u8 {
        let old_f = s.f;
        s.f = 0;
        old_f
    }
    spec take_f {
        include Equal { r: result, v: old(s.get_f()) };
    }

    fun n<X>(w: &W<X>): u64 { w.n }
    spec n {
        include Equal { r: result, v: w.n_of() };
    }

    spec Bounded {
        include AtMost { a: s.get_f(), b: lim };
    }

    fun make(s: S): Bounded { Bounded { s, lim: (s.f as u64) } }

    fun make_incorrect(s: S): Bounded { Bounded { s, lim: 0 } }

    fun applied_f(s: &S): u64 { (s.f as u64) }
    fun applied_f_again(s: &S): u64 { (s.f as u64) }
    fun applied_f_off_incorrect(s: &S): u64 { (s.f as u64) + 1 }
    spec module {
        apply Equal { r: result, v: s.get_f() } to applied_*;
    }
}
