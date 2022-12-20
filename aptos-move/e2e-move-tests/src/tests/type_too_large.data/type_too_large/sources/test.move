module 0xbeef::test {

    struct A0 has drop { a: u64 }
    struct A1 has drop { a: A0, b: A0 }
    struct A2 has drop { a: A1, b: A1 }
    struct A3 has drop { a: A2, b: A2 }
    struct A4 has drop { a: A3, b: A3 }
    struct A5 has drop { a: A4, b: A4 }
    struct A6 has drop { a: A5, b: A5 }
    struct A7 has drop { a: A6, b: A6 }
    struct A8 has drop { a: A7, b: A7 }
    struct A9 has drop { a: A8, b: A8 }
    struct A10 has drop { a: A9, b: A9 }
    struct A11 has drop { a: A10, b: A10 }
    struct A12 has drop { a: A11, b: A11 }
    struct A13 has drop { a: A12, b: A12 }
    struct A14 has drop { a: A13, b: A13 }
    struct A15 has drop { a: A14, b: A14 }
    struct A16 has drop { a: A15, b: A15 }
    struct A17 has drop { a: A16, b: A16 }
    struct A18 has drop { a: A17, b: A17 }
    struct A19 has drop { a: A18, b: A18 }
    struct A20 has drop { a: A19, b: A19 }
    struct A21 has drop { a: A20, b: A20 }
    struct A22 has drop { a: A21, b: A21 }
    struct A23 has drop { a: A22, b: A22 }
    struct A24 has drop { a: A23, b: A23 }
    struct A25 has drop { a: A24, b: A24 }
    struct A26 has drop { a: A25, b: A25 }
    struct A27 has drop { a: A26, b: A26 }
    struct A28 has drop { a: A27, b: A27 }
    struct A29 has drop { a: A28, b: A28 }
    struct A30 has drop { a: A29, b: A29 }
    struct A31 has drop { a: A30, b: A30 }
    struct A32 has drop { a: A31, b: A31 }
    struct A33 has drop { a: A32, b: A32 }
    struct A34 has drop { a: A33, b: A33 }
    struct A35 has drop { a: A34, b: A34 }
    struct A36 has drop { a: A35, b: A35 }
    struct A37 has drop { a: A36, b: A36 }
    struct A38 has drop { a: A37, b: A37 }
    struct A39 has drop { a: A38, b: A38 }
    struct A40 has drop { a: A39, b: A39 }
    struct A41 has drop { a: A40, b: A40 }
    struct A42 has drop { a: A41, b: A41 }
    struct A43 has drop { a: A42, b: A42 }
    struct A44 has drop { a: A43, b: A43 }
    struct A45 has drop { a: A44, b: A44 }
    struct A46 has drop { a: A45, b: A45 }
    struct A47 has drop { a: A46, b: A46 }
    struct A48 has drop { a: A47, b: A47 }
    struct A49 has drop { a: A48, b: A48 }
    struct A50 has drop { a: A49, b: A49 }
    struct A51 has drop { a: A50, b: A50 }
    struct A52 has drop { a: A51, b: A51 }
    struct A53 has drop { a: A52, b: A52 }
    struct A54 has drop { a: A53, b: A53 }
    struct A55 has drop { a: A54, b: A54 }
    struct A56 has drop { a: A55, b: A55 }
    struct A57 has drop { a: A56, b: A56 }
    struct A58 has drop { a: A57, b: A57 }
    struct A59 has drop { a: A58, b: A58 }
    struct A60 has drop { a: A59, b: A59 }
    struct A61 has drop { a: A60, b: A60 }
    struct A62 has drop { a: A61, b: A61 }
    struct A63 has drop { a: A62, b: A62 }

    use std::bcs; use std::vector;

    public entry fun run() {
      bcs::to_bytes<vector<A63>>(&vector::empty());
    }
}
