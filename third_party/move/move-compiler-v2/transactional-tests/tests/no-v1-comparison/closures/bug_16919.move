//# publish
module 0x9A2D1C77326B441F3C4D46BA03C92B5DF09D75609AFDF1C491CD4529080A0385::Module0 {
    struct Struct0(bool, bool) has copy, drop ;
    struct Struct1 has copy, drop {
        field2: u8,
        field3: Struct0,
    }
    enum Enum0 has copy, drop {
        Variant0 {
            field4: bool,
        },
        Variant1 {
            field5: | (| (| | has copy+drop), (| | has copy+drop), (| | has copy+drop), (| | has copy+drop) | has copy+drop), (| (| | has copy+drop), (| | has copy+drop), (| | has copy+drop), (| | has copy+drop) | has copy+drop) | ((u16, bool) ) has copy+drop,
        },
    }
    enum Enum1 has copy, drop {
        Variant2 {
            field6: u32,
            field7: u32,
        },
        Variant3 {
            field8: u32,
            field9: u32,
        },
        Variant4 {
            field10: u32,
            field11: u32,
        },
    }
    public fun function1( var0: &mut Enum1, var1: &mut Enum1, var2: &mut Enum1, var3: &mut Enum1) { /* _block0 */
    }
    public fun function2( var12: &Enum0, var13: Struct0, var14: Enum1, var15: Struct0, var16: | u32 | has copy+drop): u8 { /* _block1 */
        *( &( Enum0::Variant1 { field5: | var17: | (| | has copy+drop), (| | has copy+drop), (| | has copy+drop), (| | has copy+drop) | has copy+drop, var18: | (| | has copy+drop), (| | has copy+drop), (| | has copy+drop), (| | has copy+drop) | has copy+drop | { /* _block2 */
                        ( 41988u16, false,)
            }
        }
        )
        );
        let var19 = *( &( false));
        let () = ( function1) ( &mut ( var14),
            match ( Enum1::Variant4 { field10: ( ( 3226759902u32 + 19457u32) | 653762092u32), field11: *( &( 1822826693u32))}) {
                Enum1::Variant4 {field10: var24, ..} => { /* _block6 */
                    &mut ( Enum1::Variant2 { field6: 710633453u32, field7: 1761791860u32})
                },
                Enum1::Variant2 {field6: var20, field7: var21,} => { /* _block13 */
                        let var35: | bool, Struct1, Enum0 | (u16 ) has copy+drop = match ( Enum1::Variant4 { field10: ( 1560922432u32 | 271770460u32), field11: 1499701105u32}) {
                            Enum1::Variant3 {field8: var38, field9: var39,} => | var42: bool, var43: Struct1, var44: Enum0 | { /* _block18 */ ( 18858u16 ^ 18170u16)},
                            Enum1::Variant2 {field6: var36, ..} => { /* _block19 */
                                    | var48: bool, var49: Struct1, var50: Enum0 | { /* _block20 */ 39966u16}
                            },
                            _ => {
                                    | var59: bool, var60: Struct1, var61: Enum0 | { /* _block24 */ 6707u16}
                            }
                        };

                        match ( Enum1::Variant4 { field10: 1511514864u32, field11: ( 3357996168u32 << 10u8)}) {
                            Enum1::Variant4 {field11: var67, ..} => { /* _block29 */
                                    &mut ( Enum1::Variant2 { field6: 1159191745u32, field7: 2226907110u32})
                            },
                            _ => &mut ( var14)
                        };
                        &mut ( var14)
                },
                Enum1::Variant3 {field8: var22, field9: var23,} => match ( *( &( Enum1::Variant3 { field8: 69980731u32, field9: 65010u32}))) {
                        Enum1::Variant3 {field8: var70, ..} => &mut ( var14),
                        Enum1::Variant2 {field7: var69, ..} => &mut ( var14),
                        _ => { /* _block35 */
                                let () = ( function1) ( &mut ( var14), &mut ( Enum1::Variant2 { field6: 2733676076u32, field7: 662338862u32}), &mut ( var14), &mut ( var14));
                                let var77 = Enum1::Variant2 { field6: ( 1027423549u32 - 15677u32), field7: ( 1027423549u32 - 15677u32)};
                                &mut ( Enum1::Variant2 { field6: 2560137368u32, field7: 2560137368u32})
                        }
                }
            },
            match ( Enum1::Variant4 { field10: ( ( 2560137368u32 * 6296u32) * 6296u32), field11: ( ( 2560137368u32 * 6296u32) * 6296u32)}) {
                Enum1::Variant4 {field10: var82, field11: var83,} => match ( Enum1::Variant4 { field10: ( 2560137368u32 * 6296u32), field11: ( 2560137368u32 * 6296u32)}) {
                        Enum1::Variant4 {field10: var88, field11: var89,} => { /* _block42 */
                                &mut ( var14)
                        },
                        Enum1::Variant2 {field6: var84, field7: var85,} => &mut ( Enum1::Variant2 { field6: 537516388u32, field7: 3026101499u32}),
                        Enum1::Variant3 {field8: var86, field9: var87,} => &mut ( Enum1::Variant2 { field6: 791621423u32, field7: 791621423u32}),
                },
                Enum1::Variant2 {field6: var78, field7: var79,} => { /* _block44 */
                        let Struct0(var117, ..) = match ( Enum0::Variant1 { field5: | var121: | (| | has copy+drop), (| | has copy+drop), (| | has copy+drop), (| | has copy+drop) | has copy+drop, var122: | (| | has copy+drop), (| | has copy+drop), (| | has copy+drop), (| | has copy+drop) | has copy+drop | { /* _block47 */
                                            ( 12079u16, true,)
                                }
                            }
                        ) {
                            Enum0::Variant1 {..} => { /* _block48 */
                                    Struct0 (
                                        true,true,
                                    )
                            },
                            Enum0::Variant0 {..} => { /* _block49 */
                                    Struct0 (
                                        true,true,
                                    )
                            }
                        };
                        let var141 = ( *( &( Struct1 {field2: 157u8, field3:  Struct0 (true,true,),})) !=
                            Struct1 {
                                field2: ( 100u8 & 47u8),
                                field3:  Struct0 (
                                        true,true,
                                ),
                            }
                        );
                        let Struct0(var143, ..) = match ( Enum0::Variant1 { field5: | var147: | (| | has copy+drop), (| | has copy+drop), (| | has copy+drop), (| | has copy+drop) | has copy+drop, var148: | (| | has copy+drop), (| | has copy+drop), (| | has copy+drop), (| | has copy+drop) | has copy+drop | { /* _block52 */
                                            ( 12079u16, true,)
                                }
                            }
                        ) {
                            Enum0::Variant1 {..} => { /* _block53 */
                                    let Struct0(var150, ..) = Struct0 (
                                        true,true,
                                    );
                                    let Struct0(var153, ..) = Struct0 (
                                        true,true,
                                    );
                                    let Struct0(var156, ..) = Struct0 (
                                        true,true,
                                    );
                                    Struct0 (
                                        true,true,
                                    )
                            },
                            Enum0::Variant0 {..} => { /* _block54 */
                                    let Struct0(var159, ..) = Struct0 (
                                        true,true,
                                    );
                                    let Struct0(var162, ..) = Struct0 (
                                        true,true,
                                    );
                                    let Struct0(var165, ..) = Struct0 (
                                        true,true,
                                    );
                                    Struct0 (
                                        true,true,
                                    )
                            }
                        };
                        match ( Enum0::Variant1 { field5: | var169: | (| | has copy+drop), (| | has copy+drop), (| | has copy+drop), (| | has copy+drop) | has copy+drop, var170: | (| | has copy+drop), (| | has copy+drop), (| | has copy+drop), (| | has copy+drop) | has copy+drop | { /* _block57 */
                                            ( 12079u16, true,)
                                }
                            }
                        ) {
                            Enum0::Variant1 {..} => { /* _block58 */
                                    let Struct0(var172, ..) = Struct0 (
                                        true,true,
                                    );
                                    let Struct0(var175, ..) = Struct0 (
                                        true,true,
                                    );
                                    let Struct0(var178, ..) = Struct0 (
                                        true,true,
                                    );
                                    &mut ( var14)
                            },
                            Enum0::Variant0 {..} => &mut ( var14)
                        }
                },
                Enum1::Variant3 {field8: var80, field9: var81,} => { /* _block59 */
                        let Struct0(var181, var182) = var13;
                        let var183 = *( &( Enum0::Variant0 { field4: false}));
                        let var184 = 1867244186u32;
                        let Struct0(.., var187) = var13;
                        match ( *( var12)) {
                            Enum0::Variant1 {..} => { /* _block62 */
                                    let var190 = Enum1::Variant2 { field6: ( 2084683531u32 >> 19u8), field7: 2881938354u32};
                                    &mut ( Enum1::Variant2 { field6: 582746179u32, field7: 2065999912u32})
                            },
                            Enum0::Variant0 {..} => &mut ( var14)
                        }
                }
            },
            &mut ( var14)
        );
        0
    }
}
