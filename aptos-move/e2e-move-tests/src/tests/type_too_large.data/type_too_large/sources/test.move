module 0xbeef::test {

    struct A0 has drop, store { a: u64 }
    struct A1 has drop, store { a: A0, b: A0 }
    struct A2 has drop, store { a: A1, b: A1 }
    struct A3 has drop, store { a: A2, b: A2 }
    struct A4 has drop, store { a: A3, b: A3 }
    struct A5 has drop, store { a: A4, b: A4 }
    struct A6 has drop, store { a: A5, b: A5 }
    struct A7 has drop, store { a: A6, b: A6 }
    struct A8 has drop, store { a: A7, b: A7 }
    struct A9 has drop, store { a: A8, b: A8 }
    struct A10 has drop, store { a: A9, b: A9 }
    struct A11 has drop, store { a: A10, b: A10 }
    struct A12 has drop, store { a: A11, b: A11 }
    struct A13 has drop, store { a: A12, b: A12 }
    struct A14 has drop, store { a: A13, b: A13 }
    struct A15 has drop, store { a: A14, b: A14 }
    struct A16 has drop, store { a: A15, b: A15 }
    struct A17 has drop, store { a: A16, b: A16 }
    struct A18 has drop, store { a: A17, b: A17 }
    struct A19 has drop, store { a: A18, b: A18 }
    struct A20 has drop, store { a: A19, b: A19 }
    struct A21 has drop, store { a: A20, b: A20 }
    struct A22 has drop, store { a: A21, b: A21 }
    struct A23 has drop, store { a: A22, b: A22 }
    struct A24 has drop, store { a: A23, b: A23 }
    struct A25 has drop, store { a: A24, b: A24 }
    struct A26 has drop, store { a: A25, b: A25 }
    struct A27 has drop, store { a: A26, b: A26 }
    struct A28 has drop, store { a: A27, b: A27 }
    struct A29 has drop, store { a: A28, b: A28 }
    struct A30 has drop, store { a: A29, b: A29 }
    struct A31 has drop, store { a: A30, b: A30 }
    struct A32 has drop, store { a: A31, b: A31 }
    struct A33 has drop, store { a: A32, b: A32 }
    struct A34 has drop, store { a: A33, b: A33 }
    struct A35 has drop, store { a: A34, b: A34 }
    struct A36 has drop, store { a: A35, b: A35 }
    struct A37 has drop, store { a: A36, b: A36 }
    struct A38 has drop, store { a: A37, b: A37 }
    struct A39 has drop, store { a: A38, b: A38 }
    struct A40 has drop, store { a: A39, b: A39 }
    struct A41 has drop, store { a: A40, b: A40 }
    struct A42 has drop, store { a: A41, b: A41 }
    struct A43 has drop, store { a: A42, b: A42 }
    struct A44 has drop, store { a: A43, b: A43 }
    struct A45 has drop, store { a: A44, b: A44 }
    struct A46 has drop, store { a: A45, b: A45 }
    struct A47 has drop, store { a: A46, b: A46 }
    struct A48 has drop, store { a: A47, b: A47 }
    struct A49 has drop, store { a: A48, b: A48 }
    struct A50 has drop, store { a: A49, b: A49 }
    struct A51 has drop, store { a: A50, b: A50 }
    struct A52 has drop, store { a: A51, b: A51 }
    struct A53 has drop, store { a: A52, b: A52 }
    struct A54 has drop, store { a: A53, b: A53 }
    struct A55 has drop, store { a: A54, b: A54 }
    struct A56 has drop, store { a: A55, b: A55 }
    struct A57 has drop, store { a: A56, b: A56 }
    struct A58 has drop, store { a: A57, b: A57 }
    struct A59 has drop, store { a: A58, b: A58 }
    struct A60 has drop, store { a: A59, b: A59 }
    struct A61 has drop, store { a: A60, b: A60 }
    struct A62 has drop, store { a: A61, b: A61 }
    struct A63 has drop, store { a: A62, b: A62 }

    struct Store<T: store> has key {
        data: vector<T>,
    }

    public entry fun run() {
        // Note: this type has a very deeply nested layout. When VM tries to get the resource, the layout construction
        // will fail.
        assert!(!exists<Store<A63>>(@0x123));
    }
}
