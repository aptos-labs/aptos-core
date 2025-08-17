module 0x42::LambdaParam {
    public fun inline_apply(f: |u64|u64 has drop, b: u64) : u64 {
	f(b)
    }

    public fun inline_apply2(f: |u64|u64 has drop, b: u64) : u64 {
	inline_apply(f, b)
    }

    public fun inline_apply3(f: |u64|u64 has drop, b: u64) : u64 {
	inline_apply4(f, b)
    }

    public fun inline_apply4(_f: |u64|u64 has drop, b: u64) : u64 {
	b
    }

    fun test_lambda_symbol_param1() {
	let a = inline_apply2(|x: u64| x, 3);
	assert!(a == 3, 0);
    }

    fun test_lambda_symbol_param2() {
	let a = inline_apply2(|x: u64| x, 3);
	assert!(a == 3, 0);
	let b = inline_apply(|x: u64| x, 3);
	assert!(b == 3, 0);
	let b = inline_apply3(|x: u64| x, 3);
	assert!(b == 3, 0);
    }
}
