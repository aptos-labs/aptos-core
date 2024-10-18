module 0x42::LambdaReturn {
    public inline fun inline_apply2(f: |u64|u64, b: u64) : u64 {
	return f(b)
    }

    fun test_lambda_symbol_param() {
	let a = inline_apply2(|x: u64| { x }, 3);
	assert!(a == 3, 0);
    }
}
