// also_include_for: prophecy
module 0x42::prophecy_c3 {
    use std::vector;

    struct Inner has drop { x: u64 }
    struct Outer has drop { inner: Inner, y: u64 }

    // Nested field borrow: a chain of two field-on-reference borrows.
    fun nested_field(): (u64, u64) {
        let o = Outer { inner: Inner { x: 1 }, y: 2 };
        let r = &mut o.inner.x;
        *r = 9;
        (o.inner.x, o.y)
    }
    spec nested_field {
        ensures result_1 == 9;
        ensures result_2 == 2;
    }

    // Vector element borrow through the native vector::borrow_mut.
    fun vec_elem(): u64 {
        let v = vector[10, 20, 30];
        let r = vector::borrow_mut(&mut v, 1);
        *r = 99;
        *vector::borrow(&v, 1)
    }
    spec vec_elem {
        ensures result == 99;
    }
}
