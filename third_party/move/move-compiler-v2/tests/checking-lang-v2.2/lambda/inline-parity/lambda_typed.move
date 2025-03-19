module 0x42::LambdaTest1 {
    public inline fun inline_mul(a: u64, b: u64): u64 {
	a * b
    }

    public inline fun inline_apply1(f: |u64|u64, b: u64) : u64 {
	inline_mul(f(b) + 1, inline_mul(3, 4))
    }

    public inline fun inline_apply(f: |u64|u64, b: u64) : u64 {
	f(b)
    }
}

module 0x42::LambdaTest2 {
    use 0x42::LambdaTest1;
    use std::vector;

    public inline fun foreach<T>(v: &vector<T>, action: |&T|) { // expected to be not implemented
        let i = 0;
        while (i < vector::length(v)) {
            action(vector::borrow(v, i));
            i = i + 1;
        }
    }

    public fun test_inline_lambda() {
	let v = vector[1, 2, 3];
	let product = 1;
	foreach(&v, |e: &u64| product = LambdaTest1::inline_mul(product, *e));
    }

    public inline fun inline_apply2(g: |u64|u64, c: u64) : u64 {
	LambdaTest1::inline_apply1(|z: u64|z, g(LambdaTest1::inline_mul(c, LambdaTest1::inline_apply(|x: u64|x, 3)))) + 2
    }

    public inline fun inline_apply3(g: |u64|u64, c: u64) : u64 {
	LambdaTest1::inline_apply1(g,
	    LambdaTest1::inline_mul(c, LambdaTest1::inline_apply(|x:u64| {
		LambdaTest1::inline_apply(|y: u64|y, x)
	    },
		3))) + 4
    }
}

module 0x42::LambdaTest {
    use 0x42::LambdaTest2;

    public inline fun inline_apply(f: |u64|u64, b: u64) : u64 {
	f(b)
    }

    public inline fun inline_apply_test() : u64 {
	LambdaTest2::inline_apply2(|x: u64| x + 1, 3) +
	LambdaTest2::inline_apply2(|x: u64| x * x, inline_apply(|y: u64|y, 3))
    }

    fun test_lambda() {
	let a = inline_apply_test();
	assert!(a == 1, 0);
    }
}
