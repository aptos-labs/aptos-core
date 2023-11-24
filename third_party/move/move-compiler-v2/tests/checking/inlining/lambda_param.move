module 0x42::LambdaParam {
    public inline fun inline_apply(f: |u64|u64, b: u64) : u64 {
	f(b)
    }

    public inline fun inline_apply2(f: |u64|u64, b: u64) : u64 {
	inline_apply(f, b)
    }

    fun test_lambda_symbol_param() {
	let a = inline_apply2(|x| x, 3);
	assert!(a == 3, 0);
    }
}
