module 0x99::FiledAccess {
    struct Inner has copy, drop {
        value: u64,
    }

    struct Outer has copy, drop {
        inner: Inner,
    }

    fun get_value(outer: &Outer): u64 {
        outer.inner.value
    }

    fun set_value(outer: &mut Outer, new_value: u64) {
        outer.inner.value = new_value;
    }

    fun get_inner_value1(inner: &Inner): u64 {
        inner.value
    }

    fun get_inner_value2(inner: &Inner): u64 {
        inner.value
    }

    // `arg1.inner.value` cannot be reused due to the mutation between the two accesses
    fun test_field_access(arg1: Outer, arg2: u64): u64 {
        let x = arg1.inner.value;
        arg1.inner.value += 1;
        x + arg2 + arg1.inner.value
    }

    // `arg1.inner.value` cannot be reused due to the mutation between the two accesses
    fun test_field_access_ref(arg1: Outer, arg2: u64): u64 {
        let x = arg1.inner.value;
        let ref = &mut arg1.inner.value;
        *ref += 1;
        x + arg2 + arg1.inner.value
    }

    // `set_value(&mut arg1, arg2)` may modify `arg1.inner.value`, so the two calls cannot be reused
    fun test_field_access_mut_ref(arg1: Outer, arg2: u64): u64 {
        set_value(&mut arg1, arg2);
        set_value(&mut arg1, arg2);
        arg1.inner.value
    }
}
