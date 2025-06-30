module 0x99::basic_enum {
    #[persistent]
    fun increment_by_one(x: &mut u64): u64 { *x = *x + 1; *x }

    enum FV<T> has key {
        V1 { v1: |&mut T|T has copy+store},
    }

    fun test_fun_vec(s: &signer) {
        // not ok case: cannot store functions without `store`
        let f1: |&mut u64|u64 has copy = increment_by_one;
        let v1 = FV::V1{v1: f1};
        move_to(s, v1);
    }
}
