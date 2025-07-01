//# publish
module 0xCAFE::m0 {
    public enum Enum0 has copy, drop {
        Variant0,
        Variant1 {
            field1: bool,
        },
    }

    public struct S has copy, drop {
        field2: Enum0,
    }

}

//# publish
module 0xCAFE::test_m0 {
    use 0xCAFE::m0::Enum0;
    use 0xCAFE::m0::S;

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

//# run 0xCAFE::test_m0::f0

//# publish
module 0xCAFE::m1 {
    public enum Enum0 has copy, drop {
        Variant0,
        Variant1,
    }

    public struct S has copy, drop {
        field: Enum0,
    }
}

//# publish
module 0xCAFE::test_m1 {
    use 0xCAFE::m1::Enum0;
    use 0xCAFE::m1::S;

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

//# run 0xCAFE::test_m1::f0

//# run 0xCAFE::test_m1::f1

//# publish
module 0xCAFE::m2 {
    public struct Struct1(bool) has copy, drop;

    public enum Enum0 has copy, drop {
        Variant0,
        Variant1 {
            field1: bool,
        },
    }

    public enum Enum1 has copy, drop {
        Variant2 {
            field2: Enum0,
        },
    }

}

//# publish
module 0xCAFE::test_m2 {
    use 0xCAFE::m2::Enum0;
    use 0xCAFE::m2::Enum1;
    use 0xCAFE::m2::Struct1;

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

//# run 0xCAFE::test_m2::f0

//# publish
module 0xCAFE::m3 {
    public enum Enum0 has copy, drop {
        Variant0,
        Variant1,
    }

    public struct S has copy, drop {
        field: Enum0,
    }
}

//# publish
module 0xCAFE::test_m3 {
    use 0xCAFE::m3::Enum0;
    use 0xCAFE::m3::S;

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

//# run 0xCAFE::test_m3::test
