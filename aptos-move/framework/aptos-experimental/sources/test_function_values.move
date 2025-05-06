module aptos_experimental::test_function_values {

    fun transfer_and_create_account(some_f: |u64|u64): u64 {
        some_f(3)
    }
}
