module 0xc0ffee::unnecessary_casts_test {

    const U8_VALUE: u8 = 255;
    const U64_VALUE: u64 = 1000;

    fun helper_u64(x: u64): u64 { x }
    fun helper_u8(x: u8): u8 { x }

    // ========== POSITIVE TESTS (should warn) ==========

    public fun test_cast_in_let_warn() {
        let x: u64 = 42;
        let y = (x as u64);
    }

    public fun test_cast_in_function_arg_warn() {
        let x: u64 = 100;
        let result = helper_u64(x as u64);
    }

    public fun test_cast_in_return_warn(): u64 {
        let x: u64 = 42;
        (x as u64)
    }

    public fun test_cast_in_expression_warn() {
        let a: u32 = 10;
        let b: u32 = 20;
        let result = ((a + b) as u32);
    }

    public fun test_cast_const_warn() {
        let x = (U8_VALUE as u8);
    }

    public fun test_cast_in_conditional_warn() {
        let x: u64 = 5;
        if ((x as u64) > 0) { };
    }

    public fun test_cast_in_assert_warn() {
        let x: u16 = 100;
        assert!((x as u16) > 0, 0);
    }

    public fun test_multiple_casts_warn() {
        let a: u64 = 1;
        let b: u64 = 2;
        let sum = (a as u64) + (b as u64);
    }

    public fun test_cast_nested_warn() {
        let x: u64 = 1;
        let y = ((x as u64) as u64);
    }

    // ========== NEGATIVE TESTS (should not warn) ==========

    public fun test_necessary_cast_no_warn() {
        let x: u8 = 10;
        let y = (x as u64);
    }

    public fun test_different_types_no_warn() {
        let small: u8 = 255;
        let big = (small as u256);
    }

    public fun test_const_different_type_no_warn() {
        let x = (U8_VALUE as u256);
    }

    public fun test_no_cast_no_warn() {
        let x: u64 = 42;
        let y = x;
    }

    public fun test_downcast_no_warn() {
        let big: u64 = 100;
        let small = (big as u8);
    }

    public fun test_cross_type_cast_no_warn() {
        let int_val: u32 = 500;
        let bigger = (int_val as u128);
    }

    // ========== LINT SKIP TESTS ==========

    #[lint::skip(find_unnecessary_casts)]
    public fun test_skip_function_no_warn() {
        let x: u64 = 42;
        let y = (x as u64);
    }

    #[lint::skip(find_unnecessary_casts)]
    public fun test_skip_multiple_casts_no_warn() {
        let a: u32 = 1;
        let b: u32 = 2;
        let sum = (a as u32) + (b as u32);
    }
}

// Module-level lint skip test
#[lint::skip(find_unnecessary_casts)]
module 0xc0ffee::unnecessary_casts_skip_module {

    public fun test_module_skip_no_warn() {
        let x: u64 = 100;
        let y = (x as u64);
    }

    public fun test_module_skip_multiple_no_warn() {
        let a: u8 = 5;
        let b: u8 = 10;
        let result = (a as u8) + (b as u8);
    }
}
