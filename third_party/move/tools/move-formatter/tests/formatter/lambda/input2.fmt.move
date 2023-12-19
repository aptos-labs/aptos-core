module 0x42::LambdaTest1 {
    /* Public inline function */
    public inline fun inline_mul(a: u64, b: u64): u64 {
        /* Multiply a and b */
        a * b
    }

    // Another public inline function
    public inline fun inline_apply1(f: |u64| u64, b: u64): u64 {
        /* Apply the function f to b and multiply the result by 3 and 4 */ 
        inline_mul(f(b) + 1, inline_mul(3, 4))
    }
}

module 0x42::LambdaTest2 {
    // Use statements
    use 0x42::LambdaTest1;
    use std::vector;

    // Public inline function
    public inline fun foreach<T>(v: &vector<T>, action: |&T|) {
        // Loop through the vector and apply the action to each element
        let i = 0;
        while (i < vector::length(v)) {
            action(vector::borrow(v, i));
            i = i + 1;
        }
    }
}