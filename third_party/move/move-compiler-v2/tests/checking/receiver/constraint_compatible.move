module 0x42::m {

    struct S { value: u64 }

    fun get_value(self: &S): u64 {
        self.value
    }

    inline fun apply<T>(s: &T, f: |&T|u64): u64 {
        f(s)
    }

    // The lambda parameter `x` gets both a SomeStruct{value} constraint (from
    // field access) and a SomeReceiverFunction(get_value) constraint (from
    // receiver call). These should coexist and we should be able to infer x's type.
    fun test(s: &S): u64 {
        apply(s, |x| x.value + x.get_value())
    }
}
