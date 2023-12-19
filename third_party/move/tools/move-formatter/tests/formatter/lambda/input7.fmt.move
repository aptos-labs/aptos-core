module 0x42::LambdaTest1 {
    public inline fun inline_mul(a: u64, b: u64): u64 {
        a * b
    }

    public inline fun inline_apply1(f: |u64| u64, b: u64): u64 {
        inline_mul(f(b) + 1, inline_mul(3, 4))
    }

    public inline fun inline_apply(f: |u64| u64, b: u64): u64 {
        f(b)
    }

}
module 0x42::LambdaTest2 {
    use 0x42::LambdaTest1;
    use std::vector;

    public inline fun inline_apply2(g: |u64| u64, c: u64): u64 {

        LambdaTest1::inline_apply1(
            |z| {
                let a: u64 = LambdaTest1::inline_mul(z, 1);
                let b: u64 = LambdaTest1::inline_mul(c, LambdaTest1::inline_apply(|x| x, 2));
                g(a + b)
            },

            3
        ) + 4

    }
}