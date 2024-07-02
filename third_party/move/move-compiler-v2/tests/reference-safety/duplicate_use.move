module 0x8675309::M {

    fun ok(x: &u64) {
        ref(x, x)
    }

    fun fail(x: &mut u64) {
        mut_ref(x, x)
    }

    fun ref(_x: &u64, _y: &u64){}
    fun mut_ref(_x: &mut u64, _y: &mut u64){}


}
