module 0x42::test {

    fun no_optimize_vector() {
        use std::vector;
        let x = vector[vector[1, 2]];
        let _ = vector::borrow_mut(&mut (*vector::borrow(&x, 0)), 1);
    }

    fun optimize_vector() {
        use std::vector;
        let x = vector[vector[1, 2]];
        let _ = vector::borrow_mut(&mut (*vector::borrow_mut(&mut x, 0)), 1);
    }

    struct X has key, drop, copy {
        value: bool
    }

    fun no_optimize_resource(): bool acquires X {
        let x = &mut (*borrow_global<X>(@0x1));
        x.value
    }

    fun optimize_resource(): bool acquires X {
        let x = &(*borrow_global<X>(@0x1));
        x.value
    }

}
