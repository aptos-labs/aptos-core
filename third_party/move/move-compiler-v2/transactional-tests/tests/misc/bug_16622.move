//# publish
module 0xCAFE::m0 {
    enum Enum0 has copy, drop {
        Variant0,
        Variant1 {
            field1: bool,
        },
    }

    struct S has copy, drop {
        field2: Enum0,
    }

    public fun f0( ) {
        let x = S { field2: Enum0::Variant1 { field1: false} };
        *(match (x.field2) {
            Enum0::Variant0 => {
                &mut ( true)
            },
            _ => {
                &mut (x.field2.field1)
            }
        }) = true;
    }
}

//# run 0xCAFE::m0::f0

//# publish
module 0xCAFE::m1 {
    enum Enum0 has copy, drop {
        Variant0,
        Variant1,
    }

    struct S has copy, drop {
        field: Enum0,
    }

    public fun f0() {
        let x = S { field: Enum0::Variant1 };
        *(
            match (x.field) {
                Enum0::Variant1 {..} => {&mut (true)},
                _ => {
                        let y = x;
                        let _z = y.field;
                        &mut (true)
                }
            }
        ) = true;
    }

    public fun f1() {
        let x = S { field: Enum0::Variant1 };
        *(
            match ((x.field, x.field)) {
                (Enum0::Variant1 {..}, Enum0::Variant1 {..}) => {&mut (true)},
                (_, _) => {
                        let y = x;
                        let _z = y.field;
                        &mut (true)
                }
            }
        ) = true;
    }
}

//# run 0xCAFE::m1::f0

//# run 0xCAFE::m1::f1

//# publish
module 0xCAFE::m2 {
    struct Struct1(bool) has copy, drop;

    enum Enum0 has copy, drop {
        Variant0,
        Variant1 {
            field1: bool,
        },
    }

    enum Enum1 has copy, drop {
        Variant2 {
            field2: Enum0,
        },
    }

    public fun f0() {
        let x = Enum1::Variant2 { field2: Enum0::Variant1 { field1: false}};
        *(
            {
                *( &mut ( Struct1 (x.field2.field1,))) =
                match (x.field2) {
                    Enum0::Variant1 {..} => {
                        *( &( Struct1 (true)))
                    },
                    _ => {
                        let y = x;
                        let _z = y.field2;
                        *( &( Struct1 (true)))
                    }
                };
                &mut (true)
            }
        ) = *( &(true));
    }
}

//# run 0xCAFE::m2::f0

//# publish
module 0xCAFE::m3 {
    enum Enum0 has copy, drop {
        Variant0,
        Variant1,
    }

    struct S has copy, drop {
        field: Enum0,
    }

    fun f0(x: S) {
        *(
            match (x.field) {
                Enum0::Variant0 => &mut (1),
                _ => {
                    let _a = x.field;
                    &mut (1)
                }
            }
        ) = 1u8;
    }

    public fun test() {
        let x = S { field: Enum0::Variant1 };
        f0(x);
    }
}

//# run 0xCAFE::m3::test
