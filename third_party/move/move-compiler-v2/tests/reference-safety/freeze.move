module 0x42::m {

    fun ref_mut_mut(x: &mut u64, y: &mut u64) {
        x == y;
    }

    fun ref_imm_mut(x: &u64, y: &mut u64) {
        x == y;
    }

    fun ref_imm_imm(x: &u64, y: &u64) {
        x == y;
    }

    fun f1() {
        let x = 1;
        let r = &mut x;
        ref_mut_mut(r, r); // error
    }

    fun f2() {
        let x = 1;
        let r = &mut x;
        ref_imm_mut(r, r); // error
    }

    fun f3() {
        let x = 1;
        let r = &mut x;
        ref_imm_imm(r, r); // ok
    }


}
