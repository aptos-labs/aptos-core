module 0xcafe::basic_vec {
    /////////////////////////////////////////////////
    // INSTRUCTIONS:
    // * `VEC_LEN_BASE`
    // * `VEC_PACK_BASE`
    // * `VEC_PACK_PER_ELEM`
    // * `VEC_PUSH_BACK_BASE`
    // * `VEC_POP_BACK_BASE`
    // * `VEC_IMM_BORROW`
    // * `VEC_MUT_BORROW`

    use std::vector;

    struct T has copy, drop { a: u64 }
    struct S has copy, drop { a: u64, b: T }

    public entry fun calibrate_veclen() {
        let i = 0;
        while (i < 1000) {
            let _ = vector::length<u64>(&vector[1,2,3,4,5]);
            let _ = vector::length<u64>(&vector[1,2]);
            let _ = vector::length<u64>(&vector[1,2,3,4,5,6,7,8,9,10]);
            i = i + 1;
        };
        return
    }

    public entry fun calibrate_veclen_struct() {
        let i = 0;
        while (i < 1000) {
            let r = T { a: 0 };
            let s = S { a: 0, b: T { a: 0 } };
            let _ = vector::length<T>(&vector[r,r,r,r,r,r]);
            let _ = vector::length<S>(&vector[s,s,s,s,s,s]);
            let _ = vector::length<u64>(&vector[1,2,3,4,5]);
            i = i + 1;
        };
        return
    }

    public entry fun calibrate_vecpack() {
        let i = 0;
        while (i < 1000) {
            let _ = vector::empty<u64>();
            let _ = vector::empty<u64>();
            let _ = vector::empty<T>();
            let _ = vector::empty<T>();
            let _ = vector::empty<S>();
            let _ = vector::empty<S>();
            i = i + 1;
        };
        return
    }

    public entry fun calibrate_vecpack_underflow() {
        let i = 0;
        while (i < 1000) {
            let _ = vector::empty<u64>();
            let _ = vector::empty<u64>();
            let _ = vector::empty<T>();
            let _ = vector::empty<T>();
            let _ = vector::empty<S>();
            let _ = 0 - 1;
            let _ = vector::empty<S>();
            i = i + 1;
        };
        return
    }

    public entry fun calibrate_vecpack_veclen() {
        let i = 0;
        while (i < 1000) {
            let a = vector::empty<u64>();
            let _ = vector::length<u64>(&a);
            let b = vector::empty<T>();
            let _ = vector::length<T>(&b);
            let c = vector::empty<S>();
            let _ = vector::length<S>(&c);
            i = i + 1;
        };
        return
    }

    public entry fun calibrate_vecpushback() {
        let i = 0;
        while (i < 1000) {
            let a = vector::empty<u64>();
            vector::push_back(&mut a, 0);
            vector::push_back(&mut a, 1);
            vector::push_back(&mut a, 2);
            vector::push_back(&mut a, 3);
            vector::push_back(&mut a, 4);
            i = i + 1;
        };
        return
    }

    public entry fun calibrate_vecpushback_struct() {
        let i = 0;
        while (i < 1000) {
            let a = vector::empty<T>();
            vector::push_back(&mut a, T { a: 0 });
            vector::push_back(&mut a, T { a: 1 });
            vector::push_back(&mut a, T { a: 2 });
            vector::push_back(&mut a, T { a: 3 });
            vector::push_back(&mut a, T { a: 4 });
            i = i + 1;
        };
        return
    }

    public entry fun calibrate_vecpushback_nested_struct() {
        let i = 0;
        while (i < 1000) {
            let a = vector::empty<S>();
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            i = i + 1;
        };
        return
    }

    public entry fun calibrate_vecpopback() {
        let i = 0;
        while (i < 1000) {
            let a = vector::empty<u64>();
            vector::push_back(&mut a, 0);
            vector::push_back(&mut a, 1);
            vector::push_back(&mut a, 2);
            vector::push_back(&mut a, 3);
            vector::push_back(&mut a, 4);
            let _ = vector::pop_back(&mut a);
            let _ = vector::pop_back(&mut a);
            let _ = vector::pop_back(&mut a);
            let _ = vector::pop_back(&mut a);
            let _ = vector::pop_back(&mut a);
            i = i + 1;
        };
        return
    }

    public entry fun calibrate_vecpopback_struct() {
        let i = 0;
        while (i < 1000) {
            let a = vector::empty<T>();
            vector::push_back(&mut a, T { a: 0 });
            vector::push_back(&mut a, T { a: 1 });
            vector::push_back(&mut a, T { a: 2 });
            vector::push_back(&mut a, T { a: 3 });
            vector::push_back(&mut a, T { a: 4 });
            let _ = vector::pop_back(&mut a);
            let _ = vector::pop_back(&mut a);
            let _ = vector::pop_back(&mut a);
            let _ = vector::pop_back(&mut a);
            let _ = vector::pop_back(&mut a);
            i = i + 1;
        };
        return
    }

    public entry fun calibrate_vecpopback_nested_struct() {
        let i = 0;
        while (i < 1000) {
            let a = vector::empty<S>();
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            let _ = vector::pop_back(&mut a);
            let _ = vector::pop_back(&mut a);
            let _ = vector::pop_back(&mut a);
            let _ = vector::pop_back(&mut a);
            let _ = vector::pop_back(&mut a);
            i = i + 1;
        };
        return
    }

    public entry fun calibrate_vecswap() {
        let i = 0;
        while (i < 1000) {
            let a = vector::empty<u64>();
            vector::push_back(&mut a, 0);
            vector::push_back(&mut a, 1);
            vector::push_back(&mut a, 2);
            vector::push_back(&mut a, 3);
            vector::push_back(&mut a, 4);
            vector::swap(&mut a, 0, 1);
            vector::swap(&mut a, 1, 2);
            vector::swap(&mut a, 3, 4);
            i = i + 1;
        };
        return
    }

    public entry fun calibrate_vecswap_struct() {
        let i = 0;
        while (i < 1000) {
            let a = vector::empty<T>();
            vector::push_back(&mut a, T { a: 0 });
            vector::push_back(&mut a, T { a: 1 });
            vector::push_back(&mut a, T { a: 2 });
            vector::push_back(&mut a, T { a: 3 });
            vector::push_back(&mut a, T { a: 4 });
            vector::swap(&mut a, 0, 1);
            vector::swap(&mut a, 1, 2);
            vector::swap(&mut a, 3, 4);
            i = i + 1;
        };
        return
    }

    public entry fun calibrate_vecswap_nested_struct() {
        let i = 0;
        while (i < 1000) {
            let a = vector::empty<S>();
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            vector::swap(&mut a, 0, 1);
            vector::swap(&mut a, 1, 2);
            vector::swap(&mut a, 3, 4);
            i = i + 1;
        };
        return
    }

    public entry fun calibrate_vec_imm_borrow() {
        let i = 0;
        while (i < 1000) {
            let a = vector::empty<u64>();
            vector::push_back(&mut a, 0);
            vector::push_back(&mut a, 1);
            vector::push_back(&mut a, 2);
            vector::push_back(&mut a, 3);
            vector::push_back(&mut a, 4);
            let _ = vector::borrow<u64>(&a, 0);
            let _ = vector::borrow<u64>(&a, 1);
            let _ = vector::borrow<u64>(&a, 2);
            let _ = vector::borrow<u64>(&a, 3);
            let _ = vector::borrow<u64>(&a, 4);
            i = i + 1;
        };
        return
    }

    public entry fun calibrate_vec_imm_borrow_struct() {
        let i = 0;
        while (i < 1000) {
            let a = vector::empty<T>();
            vector::push_back(&mut a, T { a: 0 });
            vector::push_back(&mut a, T { a: 1 });
            vector::push_back(&mut a, T { a: 2 });
            vector::push_back(&mut a, T { a: 3 });
            vector::push_back(&mut a, T { a: 4 });
            let _ = vector::borrow<T>(&a, 0);
            let _ = vector::borrow<T>(&a, 1);
            let _ = vector::borrow<T>(&a, 2);
            let _ = vector::borrow<T>(&a, 3);
            let _ = vector::borrow<T>(&a, 4);
            i = i + 1;
        };
        return
    }

    public entry fun calibrate_vec_imm_borrow_nested_struct() {
        let i = 0;
        while (i < 1000) {
            let a = vector::empty<S>();
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            let _ = vector::borrow<S>(&a, 0);
            let _ = vector::borrow<S>(&a, 1);
            let _ = vector::borrow<S>(&a, 2);
            let _ = vector::borrow<S>(&a, 3);
            let _ = vector::borrow<S>(&a, 4);
            i = i + 1;
        };
        return
    }

    public entry fun calibrate_vec_mut_borrow() {
        let i = 0;
        while (i < 1000) {
            let a = vector::empty<u64>();
            vector::push_back(&mut a, 0);
            vector::push_back(&mut a, 1);
            vector::push_back(&mut a, 2);
            vector::push_back(&mut a, 3);
            vector::push_back(&mut a, 4);
            let _ = vector::borrow_mut<u64>(&mut a, 0);
            let _ = vector::borrow_mut<u64>(&mut a, 1);
            let _ = vector::borrow_mut<u64>(&mut a, 2);
            let _ = vector::borrow_mut<u64>(&mut a, 3);
            let _ = vector::borrow_mut<u64>(&mut a, 4);
            i = i + 1;
        };
        return
    }

    public entry fun calibrate_vec_mut_borrow_struct() {
        let i = 0;
        while (i < 1000) {
            let a = vector::empty<T>();
            vector::push_back(&mut a, T { a: 0 });
            vector::push_back(&mut a, T { a: 1 });
            vector::push_back(&mut a, T { a: 2 });
            vector::push_back(&mut a, T { a: 3 });
            vector::push_back(&mut a, T { a: 4 });
            let _ = vector::borrow_mut<T>(&mut a, 0);
            let _ = vector::borrow_mut<T>(&mut a, 1);
            let _ = vector::borrow_mut<T>(&mut a, 2);
            let _ = vector::borrow_mut<T>(&mut a, 3);
            let _ = vector::borrow_mut<T>(&mut a, 4);
            i = i + 1;
        };
        return
    }

    public entry fun calibrate_vec_mut_borrow_nested_struct() {
        let i = 0;
        while (i < 1000) {
            let a = vector::empty<S>();
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            vector::push_back(&mut a, S { a: 0, b: T { a: 0 } });
            let _ = vector::borrow_mut<S>(&mut a, 0);
            let _ = vector::borrow_mut<S>(&mut a, 1);
            let _ = vector::borrow_mut<S>(&mut a, 2);
            let _ = vector::borrow_mut<S>(&mut a, 3);
            let _ = vector::borrow_mut<S>(&mut a, 4);
            i = i + 1;
        };
        return
    }
}
