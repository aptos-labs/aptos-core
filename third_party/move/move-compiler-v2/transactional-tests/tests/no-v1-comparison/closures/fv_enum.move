//# publish
module 0x66::fv_enum_basic {
    use std::signer;
    enum Action has drop {
        Noop,
        Call(|u64|u64),
    }

    fun square(x: u64): u64 { x * x }

    fun call_square(x: u64) {
        let act = Action::Call(square);
        let v = match (act) {
            Action::Call(f) => f(x),
            _ => 0
        };
        assert!(v == 49);
    }

    enum Mapper<T, R> has key {
        Id(|T|R has copy + store),
        Twice(Version<T, R>),
    }

    #[persistent]
    fun add_k_persistent(x: u64, k: u64): u64 { x + k }

    enum Version<T, R> has copy,store {
        V1 { v1: |T|R has copy + store },
    }

    fun test_enum_in_another_enum(s: &signer) {
        let k = 3;
        let add_k: |u64|u64 has copy + store = |x: u64| add_k_persistent(x, k);
        let v1 = Version::V1 { v1: add_k };
        move_to(s, Mapper::Twice(v1));
        let m = borrow_global<Mapper<u64, u64>>(signer::address_of(s));
        let v = match (m) {
            Mapper::Twice(v1) => (v1.v1)((v1.v1)(10)),
            Mapper::Id(f)    => (*f)(10),
        };
        assert!(v == 16, 99);
    }

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
                let add = vector::pop_back(&mut v1);
                let v = 3;
                let x = add(&mut v);
                assert!(v == 4, 0);
                assert!(x == 7, 1);
                vector::push_back(&mut v1, add);
                let m = FunVec::V2 { v0: 10, v1 };
                move_to(s, m);
            }
            FunVec::V2 { v0: _, v1 } => {
                vector::destroy_empty(v1);
                assert!(false, 2);
            }
        };
    }

}

//# run 0x66::fv_enum_basic::call_square --args 7

//# run 0x66::fv_enum_basic::test_enum_in_another_enum --signers 0x66

//# run 0x66::fv_enum_basic::test_fun_vec --signers 0x66
