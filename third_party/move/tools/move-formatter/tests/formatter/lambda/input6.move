/// test case: a complex expression with function calls, binops, and parenthesis that spills over 90 columns.
module 0x42::LambdaTest1 {  
    // Public inline function  
    public inline fun inline_mul(a: u64, b: u64): u64 {  
        // Multiply a and b  
        a * b  
    }  
  
    // Another public inline function  
    public inline fun inline_apply1(f: |u64|u64, b: u64) : u64 {  
        // Apply the function f to b and multiply the result by 3 and 4  
        inline_mul(inline_mul(inline_mul(f(b) + 1, inline_mul(3, 4))  ,    inline_mul(f(b) + 1, inline_mul(3, 4))  )  , iinline_mul(inline_mul(f(b) + 1, inline_mul(3, 4))  ,    inline_mul(f(b) + 1, inline_mul(3, 4))  )  )
    }  
}
