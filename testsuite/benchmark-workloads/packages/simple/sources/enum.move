module 0xABCD::enum {

    enum A1  has store { Node { x: A2, } }
    enum A2  has store { Node { x: A3, } }
    enum A3  has store { Node { x: A4, } }
    enum A4  has store { Node { x: A5, } }
    enum A5  has store { Node { x: A6, } }
    enum A6  has store { Node { x: A7, } }
    enum A7  has store { Node { x: A8, } }
    enum A8  has store { Node { x: A9, } }
    enum A9  has store { Node { x: A10, } }
    enum A10 has store { Node { x: A11, } }
    enum A11 has store { Node { x: A12, } }
    enum A12 has store { Node { x: A13, } }
    enum A13 has store { Node { x: A14, } }
    enum A14 has store { Node { x: A15, } }
    enum A15 has store { Node { x: A16, } }
    enum A16 has store { Node { x: A17, } }
    enum A17 has store { Node { x: A18, } }
    enum A18 has store { Node { x: A19, } }
    enum A19 has store { Node { x: A20, } }
    enum A20 has store { Node { x: A21, } }
    enum A21 has store { Node { x: A22, } }
    enum A22 has store { Node { x: A23, } }
    enum A23 has store { Node { x: A24, } }
    enum A24 has store { Node { x: A25, } }
    enum A25 has store { Node { x: A26, } }
    enum A26 has store { Node { x: A27, } }
    enum A27 has store { Node { x: A28, } }
    enum A28 has store { Node { x: A29, } }
    enum A29 has store { Node { x: A30, } }
    enum A30 has store { Node { x: A31, } }
    enum A31 has store { Node { x: A32, } }
    enum A32 has store { Leaf { x: u64, } }

    struct Store has key { tree: A1 }

    fun init_module(account: &signer) {
        let tree = A1::Node { x:
            A2::Node { x:
            A3::Node { x:
            A4::Node { x:
            A5::Node { x:
            A6::Node { x:
            A7::Node { x:
            A8::Node { x:
            A9::Node { x:
            A10::Node { x:
            A11::Node { x:
            A12::Node { x:
            A13::Node { x:
            A14::Node { x:
            A15::Node { x:
            A16::Node { x:
            A17::Node { x:
            A18::Node { x:
            A19::Node { x:
            A20::Node { x:
            A21::Node { x:
            A22::Node { x:
            A23::Node { x:
            A24::Node { x:
            A25::Node { x:
            A26::Node { x:
            A27::Node { x:
            A28::Node { x:
            A29::Node { x:
            A30::Node { x:
            A31::Node { x:
            A32::Leaf { x: 1 } }}}}}}}}}}}}}}}}}}}}}}}}}}}}}}};
        move_to(account, Store { tree });
    }

    /// Performs 4096 ImmBorrowVariantField instructions.
    public entry fun read_enum_variants() acquires Store {
        let store = borrow_global<Store>(@0xABCD);
        let enum = &store.tree;

        let i = 0;
        while (i < 128) {
            let A1::Node { x }  = enum;
            let A2::Node { x }  = x;
            let A3::Node { x }  = x;
            let A4::Node { x }  = x;
            let A5::Node { x }  = x;
            let A6::Node { x }  = x;
            let A7::Node { x }  = x;
            let A8::Node { x }  = x;
            let A9::Node { x }  = x;
            let A10::Node { x }  = x;
            let A11::Node { x }  = x;
            let A12::Node { x }  = x;
            let A13::Node { x }  = x;
            let A14::Node { x }  = x;
            let A15::Node { x }  = x;
            let A16::Node { x }  = x;
            let A17::Node { x }  = x;
            let A18::Node { x }  = x;
            let A19::Node { x }  = x;
            let A20::Node { x }  = x;
            let A21::Node { x }  = x;
            let A22::Node { x }  = x;
            let A23::Node { x }  = x;
            let A24::Node { x }  = x;
            let A25::Node { x }  = x;
            let A26::Node { x }  = x;
            let A27::Node { x }  = x;
            let A28::Node { x }  = x;
            let A29::Node { x }  = x;
            let A30::Node { x }  = x;
            let A31::Node { x }  = x;
            let A32::Leaf { x }  = x;

            assert!(*x == 1, 404);
            i = i + 1;
        }
    }
}
