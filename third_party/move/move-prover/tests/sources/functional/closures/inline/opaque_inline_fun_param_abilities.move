// Tests an inline function which applies its function parameter twice; this
// requires the `copy` ability on the function type for the body to be
// compiled and verified (without it, an ability error is reported).
module 0x42::opaque_inline_fun_param_abilities {

    inline fun twice(f: |u64| u64 has copy + drop, x: u64): u64 {
        f(f(x))
    }
    spec twice {
        pragma opaque;
        ensures exists m: u64: ensures_of<f>(x, m) && ensures_of<f>(m, result);
    }

    fun test_twice(x: u64): u64 {
        twice(|y| y + 1 spec { ensures result == y + 1; }, x)
    }
    spec test_twice {
        requires x < 1000;
        ensures result == x + 2;
    }
}
