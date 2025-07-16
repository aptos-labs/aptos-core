module 0x99::basic_enum {
    enum FV<T> has key {
        V1 { v1: |&mut T|T has copy+store},
    }

    fun test_fun_vec(s: &signer) {
        // not ok case: cannot put function values in storage directly
        let f1: |&mut u64|u64 has copy+store = |x| *x+1;
        let v1 = FV::V1{v1: f1};
        move_to(s, v1);
    }
}
