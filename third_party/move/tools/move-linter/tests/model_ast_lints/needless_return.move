module 0xc0ffee::m {

    use std::option::{Option, none, some};


    // Tests with only return statements in the body of the function
    public fun with_return_void(_x : bool) {
        return  // Should warn here.
    }

    public fun with_return_void_sc(_x : bool) {
        return;  // Should warn here.
    }

    public fun with_return_non_void(x : bool): Option<bool> {
        return some(!x)  // Should warn here.
    }
    // Tests with only return statements in the body of the function
    // Should warn only in the last return statement
    public fun with_return_void_bigger_body(x : bool){
        if (x) {
            return // Should not warn here.
        };

        return  // Should warn here.
    }

    public fun with_return_non_void_bigger_body(x : bool): Option<bool> {
        if (x) {
            return none<bool>(); // Should not warn here.
        };

        return some(!x)  // Should warn here.
    }


    // Without return statements - should not warn
    public fun no_return_void(_x : bool) {
        // should not warn
    }

    public fun no_return_non_void(x : bool): Option<bool> {
        if (x) {
            return none<bool>(); // Should not warn here.
        };

        some(!x)  // Should not warn here.
    }

    public fun return_tuple_multiple_params(x: bool): (u8, u8) {
        if (x) {
            return (1, 2); // Should not warn here.
        };
        (1, 2) // Should not warn here.
    }

    #[lint::skip(nonminimal_bool)]
    fun test_no_warn(): bool {
        !true  // Should not warn here.
    }

    #[lint::skip(needless_return)]
    fun test_skip(): bool {
        return true
    }
}
