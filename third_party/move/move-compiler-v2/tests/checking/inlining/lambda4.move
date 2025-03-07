module 0x8675309::M {
    public inline fun macro_result_lambda_not_allowed(): |u64| {  // expected lambda not allowed
        abort (1)
    }
    public fun fun_result_lambda_not_allowed(): |u64| {  // expected lambda not allowed
        abort (1)
    }
}
