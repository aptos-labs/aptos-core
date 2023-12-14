module NamedAddr::counter {
    use aptos_std::table::{Self, Table};

    inline fun complex_inline_function() {
        let  result = 1;
        result = result + 1;
        result = result + 2;
        result = result + 3;
        result = result + 4;
        result = result + 5;
        result = result + 6;
        result = result + 7;
        result = result + 8;
        result = result + 9;
        result = result + 10;
        result
    }

    // A simple inline function (less than or equal to 10 statements)
    inline fun simple_inline_function(x: u64) {
        x + 1
    }

    // Functions to demonstrate frequent usage of `complex_inline_function`
    public fun use_complex_function1() {
        let _ = complex_inline_function();
    }

    public fun use_complex_function2() {
        let _ = complex_inline_function();
    }

    public fun use_complex_function3() {
        let _ = complex_inline_function();
    }

    public fun use_complex_function4() {
        let _ = complex_inline_function();
    }


    // Function that uses `simple_inline_function` only once
    public fun use_simple_function() {
        let _ = simple_inline_function(3);
    }

    // Example usage of simple_inline_function in another context
    public fun another_use_simple_function() {
        let _ = simple_inline_function(4);
    }

}