module 0x66::fv_enum_basic {
    public enum Action has drop {
        Noop,
        Call(|u64|u64),
    }

    public enum Mapper<T, R> has drop {
        Id(|T|R has copy + drop + store),
        Twice(Version<T, R>),
    }

    public enum Version<T, R> has copy,store, drop {
        V1 { v1: |T|R has copy + drop + store },
    }

    public enum FunVec<T> has drop {
        V1 { v1: vector<|&mut T|T has copy + drop + store> },
        V2 { v0: u64, v1: vector<|&mut T|T has copy + drop + store> },
    }

}

module 0x66::fv_enum_basic_public {
    use 0x66::fv_enum_basic::Action;
    use 0x66::fv_enum_basic::Mapper;
    use 0x66::fv_enum_basic::Version;
    use 0x66::fv_enum_basic::FunVec;

    fun square(x: u64): u64 { x * x }

    fun call_square(x: u64) {
        let act = Action::Call(square);
        let v = match (act) {
            Action::Call(f) => f(x),
            _ => 0
        };
        assert!(v == 49);
    }

    #[persistent]
    fun add_k_persistent(x: u64, k: u64): u64 { x + k }

    fun test_enum_in_another_enum() {
        let k = 3;
        let add_k: |u64|u64 has copy + drop + store = |x: u64| add_k_persistent(x, k);
        let v1 = Version::V1 { v1: add_k };
        let v2 = Mapper::Twice(v1);
        let v = match (&mut v2) {
            Mapper::Twice(v1) => (v1.v1)((v1.v1)(10)),
            Mapper::Id(f)    => (*f)(10),
        };
        assert!(v == 16, 99);
    }

    #[persistent]
    fun add_k_persistent_ref(x: &mut u64, k: u64): u64 { *x = *x + 1; *x + k }

    fun test_fun_vec() {
        use std::vector;
        let k = 3;
        let add_k: |&mut u64|u64 has copy + store + drop = |x: &mut u64| add_k_persistent_ref(x, k);
        let v1 = FunVec::V1 { v1: vector[add_k, add_k] };
        match (v1) {
            FunVec::V1 { v1 } => {
                let add = vector::pop_back(&mut v1);
                let v = 3;
                let x = add(&mut v);
                assert!(v == 4, 0);
                assert!(x == 7, 1);
                vector::push_back(&mut v1, add);
                let _m = FunVec::V2 { v0: 10, v1 };
            }
            FunVec::V2 { v0: _, v1 } => {
                vector::destroy_empty(v1);
                assert!(false, 2);
            }
        };
    }

}
