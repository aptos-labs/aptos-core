// In verify mode, the body of an inline function with a spec block is compiled
// and must satisfy the ability rules: applying `f` twice requires `copy` on the
// function type. In normal compilation mode, the function is expanded and no
// such requirement arises.
module 0x42::retained_no_copy_fun_param {

    inline fun twice(f: |u64| u64, x: u64): u64 {
        f(f(x))
    }
    spec twice {
        pragma opaque;
        ensures exists m: u64: ensures_of<f>(x, m) && ensures_of<f>(m, result);
    }

    inline fun twice_copy(f: |u64| u64 has copy + drop, x: u64): u64 {
        f(f(x))
    }
    spec twice_copy {
        pragma opaque;
        ensures exists m: u64: ensures_of<f>(x, m) && ensures_of<f>(m, result);
    }

    fun caller(x: u64): u64 {
        twice(|y| y + 1, x) + twice_copy(|y| y + 1, x)
    }
}
