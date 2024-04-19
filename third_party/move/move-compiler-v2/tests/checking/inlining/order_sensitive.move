module 0x42::OrderSensitiveTest1 {
    public inline fun inline_fun1(a: u64, b: u64): u64 {
	a * b
    }

    public inline fun inline_fun2(a: u64, b: u64): u64 {
	inline_fun1(a, b) + 2 * inline_fun3(a, b)
    }

    public inline fun inline_fun3(a: u64, b: u64): u64 {
	a * b + 2
    }
}

module 0x42::OrderSensitiveTest2 {
    use 0x42::OrderSensitiveTest1;

    public inline fun inline_fun1(a: u64, b: u64): u64 {
	a * b + 3
    }

    public inline fun inline_fun2(a: u64, b: u64): u64 {
	OrderSensitiveTest1::inline_fun2(inline_fun1(a, b), inline_fun3(a, b))
	+ 3 * inline_fun1(a, b)
	+ 5 * inline_fun3(a, b)
    }

    public inline fun inline_fun3(a: u64, b: u64): u64 {
	a * b + 4
    }
}

module 0x42::OrderSensitiveTest3 {
    use 0x42::OrderSensitiveTest2;

    public inline fun fun1(a: u64, b: u64): u64 {
	a * b + 5
    }

    public fun fun2(a: u64, b: u64): u64 {
	OrderSensitiveTest2::inline_fun2(7 * fun1(a, b), b)
	+ 9 * fun3(a, b)
    }

    public inline fun fun3(a: u64, b: u64): u64 {
	a * b + 6
    }
}
