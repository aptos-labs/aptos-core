module 0x42::m {

    fun ref_to_ref(x: u64, y: u64) {
        &x == &y;
    }

    fun mut_ref_to_ref(x: u64, y: u64) {
        &mut x == &y;
    }

    fun mut_ref_to_mut_ref(x: u64, y: u64) {
        &mut x == &mut y;
    }

    fun mut_ref_id(x: u64) {
        &mut x == &mut x;
    }
}
