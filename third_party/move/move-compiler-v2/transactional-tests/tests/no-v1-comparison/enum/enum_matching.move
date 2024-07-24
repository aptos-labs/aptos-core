//# publish
module 0x42::m {

    enum Inner has drop {
        Inner1{ x: u64 }
        Inner2{ x: u64, y: u64 }
    }

    struct Box has drop {
        x: u64
    }

    enum Outer has drop {
        None,
        One{i: Inner},
        Two{i: Inner, b: Box},
    }

    /// Simple matching
    public fun inner_value(self: Inner): u64 {
        match (self) {
            Inner1{x} => x,
            Inner2{x, y} => x + y
        }
    }

    /// Matching with wildcard and reference
    public fun is_inner1(self: &Inner): bool {
        match (self) {
            Inner1{x: _} => true,
            _ => false
        }
    }

    /// Matching which delegates ownership
    public fun outer_value(o: Outer): u64 {
        match (o) {
            None => 0,
            // `i` is moved and consumed by `inner_value`
            One{i} => i.inner_value(),
            Two{i, b} => i.inner_value() + b.x
        }
    }

    /// Nested matching with delegation
    public fun outer_value_nested(o: Outer): u64 {
        match (o) {
            None => 0,
            // Nested match will require multiple probing steps
            One{i: Inner::Inner1{x}} => x,
            One{i} => i.inner_value(),
            Two{i, b} => i.inner_value() + b.x
        }
    }

    /// Matching with condition
    public fun outer_value_with_cond(o: Outer): u64 {
        match (o) {
            None => 0,
            // Match with condition requires probing and conversion from 'Deref(Borrow(x))` to `x`.
            One{i} if i.is_inner1() => i.inner_value() % 2,
            One{i} => i.inner_value(),
            Two{i, b} => i.inner_value() + b.x
        }
    }

    /// Matching with condition with references and wildcard
    public fun outer_value_with_cond_ref(o: &Outer): bool {
        match (o) {
            None => false,
            One{i} if i.is_inner1() => true,
            One{i} => i.is_inner1(),
            Two{i, b: _} => i.is_inner1()
        }
    }

    /// Matching with abilities and generics
    enum Option<A> has drop {
        None,
        Some{value: A}
    }

    public fun is_some<A>(x: &Option<A>): bool {
        match (x) {
            None => false,
            Some{value: _} => true
        }
    }

    public fun is_some_specialized(x: &Option<Option<u64>>): bool {
        match (x) {
            None => false,
            Some{value: Option::None} => false,
            Some{value: Option::Some{value: _}} => true,
        }
    }

    public fun is_some_dropped<A:drop>(x: Option<A>): bool {
        match (x) {
            None => false,
            _ => true
        }
    }

    // Common fields
    enum CommonFields {
        Foo{x: u64, y: u64},
        Bar{z: u64, x: u64}
    }

    fun select_common_fields(s: CommonFields): u64 {
        s.x + (match (s) { Foo{x: _, y} => y, Bar{z, x: _} => z })
    }

    // -------------------
    // Test entry points

    fun t1_is_inner1(): bool {
        is_inner1(&Inner::Inner1{x: 2})
    }

    fun t2_is_inner1(): bool {
        is_inner1(&Inner::Inner2{x: 2, y: 3})
    }

    fun t1_inner_value(): u64 {
        inner_value(Inner::Inner2{x: 2, y: 5})
    }

    fun t1_outer_value(): u64 {
        outer_value(Outer::None{})
    }

    fun t2_outer_value(): u64 {
        outer_value(Outer::One{i: Inner::Inner2{x: 1, y: 2}})
    }

    fun t3_outer_value(): u64 {
        outer_value(Outer::Two{i: Inner::Inner1{x: 1}, b: Box{x: 7}})
    }

    fun t1_outer_value_nested(): u64 {
        outer_value_nested(Outer::One{i: Inner::Inner1{x: 27}})
    }

    fun t2_outer_value_nested(): u64 {
        outer_value_nested(Outer::Two{i: Inner::Inner1{x: 5}, b: Box{x: 7}})
    }

    fun t1_outer_value_with_cond(): u64 {
        outer_value_with_cond(Outer::One{i: Inner::Inner1{x: 43}})
    }

    fun t1_outer_value_with_cond_ref(): bool {
        outer_value_with_cond_ref(&Outer::One{i: Inner::Inner1{x: 43}})
    }

    fun t1_is_some(): bool {
        is_some(&Option::None<u64>{})
    }

    fun t2_is_some(): bool {
        is_some(&Option::Some{value: 3})
    }

    fun t1_is_some_specialized(): bool {
        is_some_specialized(&Option::Some{value: Option::None{}})
    }

    fun t2_is_some_specialized(): bool {
        is_some_specialized(&Option::Some{value: Option::Some{value: 1}})
    }
}

//# run 0x42::m::t1_is_inner1

//# run 0x42::m::t2_is_inner1

//# run 0x42::m::t1_inner_value

//# run 0x42::m::t1_outer_value

//# run 0x42::m::t2_outer_value

//# run 0x42::m::t3_outer_value

//# run 0x42::m::t1_outer_value_nested

//# run 0x42::m::t2_outer_value_nested

//# run 0x42::m::t1_outer_value_with_cond

//# run 0x42::m::t1_outer_value_with_cond_ref

//# run 0x42::m::t1_is_some

//# run 0x42::m::t2_is_some

//# run 0x42::m::t1_is_some_specialized

//# run 0x42::m::t2_is_some_specialized
