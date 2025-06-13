module 0x66::fv_enum_basic {
    use std::signer;

    #[persistent]
    fun add_k_persistent_ref(x: &mut u64, k: u64): u64 { *x = *x + 1; *x + k }

    enum FunVec<T> has key {
        V1 { v1: vector<|&mut T|T has copy + store> },
        V2 { v0: u64, v1: vector<|&mut T|T has copy + store> },
    }

    fun test_fun_vec(s: &signer) {
        use std::vector;
        let k = 3;
        let add_k: |&mut u64|u64 has copy + store + drop = |x: &mut u64| add_k_persistent_ref(x, k);
        let v1 = FunVec::V1 { v1: vector[add_k, add_k] };
        move_to(s, v1);
        let m = move_from<FunVec<u64>>(signer::address_of(s));
        match (m) {
            FunVec::V1 { v1 } => {
                vector::pop_back(&mut v1);
                vector::pop_back(&mut v1);
                vector::destroy_empty(v1);
            }
            FunVec::V2 { v0: _, v1 } => {
                vector::destroy_empty(v1);
                assert!(false, 99);
            }
        };
    }

}
