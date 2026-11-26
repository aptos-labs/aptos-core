// The largest values and types V1 accepts, run on both VMs: a value of depth 128 (127 nested
// structs over a u64; copied, compared and serialized), a type of depth 20 (u8 under 19
// nested vectors) and an instantiated type of 128 nodes. Each is followed by its first
// rejected neighbour, one level or node larger: V1 rejects it with its limit error, and
// MonoMove, which enforces none of these limits, runs it.

// RUN: publish
module 0x42::deep {
    use std::bcs;
    use std::vector;
    struct S0 has copy, drop { x: u64 }
    struct S1 has copy, drop { s: S0 }
    struct S2 has copy, drop { s: S1 }
    struct S3 has copy, drop { s: S2 }
    struct S4 has copy, drop { s: S3 }
    struct S5 has copy, drop { s: S4 }
    struct S6 has copy, drop { s: S5 }
    struct S7 has copy, drop { s: S6 }
    struct S8 has copy, drop { s: S7 }
    struct S9 has copy, drop { s: S8 }
    struct S10 has copy, drop { s: S9 }
    struct S11 has copy, drop { s: S10 }
    struct S12 has copy, drop { s: S11 }
    struct S13 has copy, drop { s: S12 }
    struct S14 has copy, drop { s: S13 }
    struct S15 has copy, drop { s: S14 }
    struct S16 has copy, drop { s: S15 }
    struct S17 has copy, drop { s: S16 }
    struct S18 has copy, drop { s: S17 }
    struct S19 has copy, drop { s: S18 }
    struct S20 has copy, drop { s: S19 }
    struct S21 has copy, drop { s: S20 }
    struct S22 has copy, drop { s: S21 }
    struct S23 has copy, drop { s: S22 }
    struct S24 has copy, drop { s: S23 }
    struct S25 has copy, drop { s: S24 }
    struct S26 has copy, drop { s: S25 }
    struct S27 has copy, drop { s: S26 }
    struct S28 has copy, drop { s: S27 }
    struct S29 has copy, drop { s: S28 }
    struct S30 has copy, drop { s: S29 }
    struct S31 has copy, drop { s: S30 }
    struct S32 has copy, drop { s: S31 }
    struct S33 has copy, drop { s: S32 }
    struct S34 has copy, drop { s: S33 }
    struct S35 has copy, drop { s: S34 }
    struct S36 has copy, drop { s: S35 }
    struct S37 has copy, drop { s: S36 }
    struct S38 has copy, drop { s: S37 }
    struct S39 has copy, drop { s: S38 }
    struct S40 has copy, drop { s: S39 }
    struct S41 has copy, drop { s: S40 }
    struct S42 has copy, drop { s: S41 }
    struct S43 has copy, drop { s: S42 }
    struct S44 has copy, drop { s: S43 }
    struct S45 has copy, drop { s: S44 }
    struct S46 has copy, drop { s: S45 }
    struct S47 has copy, drop { s: S46 }
    struct S48 has copy, drop { s: S47 }
    struct S49 has copy, drop { s: S48 }
    struct S50 has copy, drop { s: S49 }
    struct S51 has copy, drop { s: S50 }
    struct S52 has copy, drop { s: S51 }
    struct S53 has copy, drop { s: S52 }
    struct S54 has copy, drop { s: S53 }
    struct S55 has copy, drop { s: S54 }
    struct S56 has copy, drop { s: S55 }
    struct S57 has copy, drop { s: S56 }
    struct S58 has copy, drop { s: S57 }
    struct S59 has copy, drop { s: S58 }
    struct S60 has copy, drop { s: S59 }
    struct S61 has copy, drop { s: S60 }
    struct S62 has copy, drop { s: S61 }
    struct S63 has copy, drop { s: S62 }
    struct S64 has copy, drop { s: S63 }
    struct S65 has copy, drop { s: S64 }
    struct S66 has copy, drop { s: S65 }
    struct S67 has copy, drop { s: S66 }
    struct S68 has copy, drop { s: S67 }
    struct S69 has copy, drop { s: S68 }
    struct S70 has copy, drop { s: S69 }
    struct S71 has copy, drop { s: S70 }
    struct S72 has copy, drop { s: S71 }
    struct S73 has copy, drop { s: S72 }
    struct S74 has copy, drop { s: S73 }
    struct S75 has copy, drop { s: S74 }
    struct S76 has copy, drop { s: S75 }
    struct S77 has copy, drop { s: S76 }
    struct S78 has copy, drop { s: S77 }
    struct S79 has copy, drop { s: S78 }
    struct S80 has copy, drop { s: S79 }
    struct S81 has copy, drop { s: S80 }
    struct S82 has copy, drop { s: S81 }
    struct S83 has copy, drop { s: S82 }
    struct S84 has copy, drop { s: S83 }
    struct S85 has copy, drop { s: S84 }
    struct S86 has copy, drop { s: S85 }
    struct S87 has copy, drop { s: S86 }
    struct S88 has copy, drop { s: S87 }
    struct S89 has copy, drop { s: S88 }
    struct S90 has copy, drop { s: S89 }
    struct S91 has copy, drop { s: S90 }
    struct S92 has copy, drop { s: S91 }
    struct S93 has copy, drop { s: S92 }
    struct S94 has copy, drop { s: S93 }
    struct S95 has copy, drop { s: S94 }
    struct S96 has copy, drop { s: S95 }
    struct S97 has copy, drop { s: S96 }
    struct S98 has copy, drop { s: S97 }
    struct S99 has copy, drop { s: S98 }
    struct S100 has copy, drop { s: S99 }
    struct S101 has copy, drop { s: S100 }
    struct S102 has copy, drop { s: S101 }
    struct S103 has copy, drop { s: S102 }
    struct S104 has copy, drop { s: S103 }
    struct S105 has copy, drop { s: S104 }
    struct S106 has copy, drop { s: S105 }
    struct S107 has copy, drop { s: S106 }
    struct S108 has copy, drop { s: S107 }
    struct S109 has copy, drop { s: S108 }
    struct S110 has copy, drop { s: S109 }
    struct S111 has copy, drop { s: S110 }
    struct S112 has copy, drop { s: S111 }
    struct S113 has copy, drop { s: S112 }
    struct S114 has copy, drop { s: S113 }
    struct S115 has copy, drop { s: S114 }
    struct S116 has copy, drop { s: S115 }
    struct S117 has copy, drop { s: S116 }
    struct S118 has copy, drop { s: S117 }
    struct S119 has copy, drop { s: S118 }
    struct S120 has copy, drop { s: S119 }
    struct S121 has copy, drop { s: S120 }
    struct S122 has copy, drop { s: S121 }
    struct S123 has copy, drop { s: S122 }
    struct S124 has copy, drop { s: S123 }
    struct S125 has copy, drop { s: S124 }
    struct S126 has copy, drop { s: S125 }
    struct S127 has copy, drop { s: S126 }
    fun mk0(): S0 { S0 { x: 7 } }
    fun mk1(): S1 { S1 { s: mk0() } }
    fun mk2(): S2 { S2 { s: mk1() } }
    fun mk3(): S3 { S3 { s: mk2() } }
    fun mk4(): S4 { S4 { s: mk3() } }
    fun mk5(): S5 { S5 { s: mk4() } }
    fun mk6(): S6 { S6 { s: mk5() } }
    fun mk7(): S7 { S7 { s: mk6() } }
    fun mk8(): S8 { S8 { s: mk7() } }
    fun mk9(): S9 { S9 { s: mk8() } }
    fun mk10(): S10 { S10 { s: mk9() } }
    fun mk11(): S11 { S11 { s: mk10() } }
    fun mk12(): S12 { S12 { s: mk11() } }
    fun mk13(): S13 { S13 { s: mk12() } }
    fun mk14(): S14 { S14 { s: mk13() } }
    fun mk15(): S15 { S15 { s: mk14() } }
    fun mk16(): S16 { S16 { s: mk15() } }
    fun mk17(): S17 { S17 { s: mk16() } }
    fun mk18(): S18 { S18 { s: mk17() } }
    fun mk19(): S19 { S19 { s: mk18() } }
    fun mk20(): S20 { S20 { s: mk19() } }
    fun mk21(): S21 { S21 { s: mk20() } }
    fun mk22(): S22 { S22 { s: mk21() } }
    fun mk23(): S23 { S23 { s: mk22() } }
    fun mk24(): S24 { S24 { s: mk23() } }
    fun mk25(): S25 { S25 { s: mk24() } }
    fun mk26(): S26 { S26 { s: mk25() } }
    fun mk27(): S27 { S27 { s: mk26() } }
    fun mk28(): S28 { S28 { s: mk27() } }
    fun mk29(): S29 { S29 { s: mk28() } }
    fun mk30(): S30 { S30 { s: mk29() } }
    fun mk31(): S31 { S31 { s: mk30() } }
    fun mk32(): S32 { S32 { s: mk31() } }
    fun mk33(): S33 { S33 { s: mk32() } }
    fun mk34(): S34 { S34 { s: mk33() } }
    fun mk35(): S35 { S35 { s: mk34() } }
    fun mk36(): S36 { S36 { s: mk35() } }
    fun mk37(): S37 { S37 { s: mk36() } }
    fun mk38(): S38 { S38 { s: mk37() } }
    fun mk39(): S39 { S39 { s: mk38() } }
    fun mk40(): S40 { S40 { s: mk39() } }
    fun mk41(): S41 { S41 { s: mk40() } }
    fun mk42(): S42 { S42 { s: mk41() } }
    fun mk43(): S43 { S43 { s: mk42() } }
    fun mk44(): S44 { S44 { s: mk43() } }
    fun mk45(): S45 { S45 { s: mk44() } }
    fun mk46(): S46 { S46 { s: mk45() } }
    fun mk47(): S47 { S47 { s: mk46() } }
    fun mk48(): S48 { S48 { s: mk47() } }
    fun mk49(): S49 { S49 { s: mk48() } }
    fun mk50(): S50 { S50 { s: mk49() } }
    fun mk51(): S51 { S51 { s: mk50() } }
    fun mk52(): S52 { S52 { s: mk51() } }
    fun mk53(): S53 { S53 { s: mk52() } }
    fun mk54(): S54 { S54 { s: mk53() } }
    fun mk55(): S55 { S55 { s: mk54() } }
    fun mk56(): S56 { S56 { s: mk55() } }
    fun mk57(): S57 { S57 { s: mk56() } }
    fun mk58(): S58 { S58 { s: mk57() } }
    fun mk59(): S59 { S59 { s: mk58() } }
    fun mk60(): S60 { S60 { s: mk59() } }
    fun mk61(): S61 { S61 { s: mk60() } }
    fun mk62(): S62 { S62 { s: mk61() } }
    fun mk63(): S63 { S63 { s: mk62() } }
    fun mk64(): S64 { S64 { s: mk63() } }
    fun mk65(): S65 { S65 { s: mk64() } }
    fun mk66(): S66 { S66 { s: mk65() } }
    fun mk67(): S67 { S67 { s: mk66() } }
    fun mk68(): S68 { S68 { s: mk67() } }
    fun mk69(): S69 { S69 { s: mk68() } }
    fun mk70(): S70 { S70 { s: mk69() } }
    fun mk71(): S71 { S71 { s: mk70() } }
    fun mk72(): S72 { S72 { s: mk71() } }
    fun mk73(): S73 { S73 { s: mk72() } }
    fun mk74(): S74 { S74 { s: mk73() } }
    fun mk75(): S75 { S75 { s: mk74() } }
    fun mk76(): S76 { S76 { s: mk75() } }
    fun mk77(): S77 { S77 { s: mk76() } }
    fun mk78(): S78 { S78 { s: mk77() } }
    fun mk79(): S79 { S79 { s: mk78() } }
    fun mk80(): S80 { S80 { s: mk79() } }
    fun mk81(): S81 { S81 { s: mk80() } }
    fun mk82(): S82 { S82 { s: mk81() } }
    fun mk83(): S83 { S83 { s: mk82() } }
    fun mk84(): S84 { S84 { s: mk83() } }
    fun mk85(): S85 { S85 { s: mk84() } }
    fun mk86(): S86 { S86 { s: mk85() } }
    fun mk87(): S87 { S87 { s: mk86() } }
    fun mk88(): S88 { S88 { s: mk87() } }
    fun mk89(): S89 { S89 { s: mk88() } }
    fun mk90(): S90 { S90 { s: mk89() } }
    fun mk91(): S91 { S91 { s: mk90() } }
    fun mk92(): S92 { S92 { s: mk91() } }
    fun mk93(): S93 { S93 { s: mk92() } }
    fun mk94(): S94 { S94 { s: mk93() } }
    fun mk95(): S95 { S95 { s: mk94() } }
    fun mk96(): S96 { S96 { s: mk95() } }
    fun mk97(): S97 { S97 { s: mk96() } }
    fun mk98(): S98 { S98 { s: mk97() } }
    fun mk99(): S99 { S99 { s: mk98() } }
    fun mk100(): S100 { S100 { s: mk99() } }
    fun mk101(): S101 { S101 { s: mk100() } }
    fun mk102(): S102 { S102 { s: mk101() } }
    fun mk103(): S103 { S103 { s: mk102() } }
    fun mk104(): S104 { S104 { s: mk103() } }
    fun mk105(): S105 { S105 { s: mk104() } }
    fun mk106(): S106 { S106 { s: mk105() } }
    fun mk107(): S107 { S107 { s: mk106() } }
    fun mk108(): S108 { S108 { s: mk107() } }
    fun mk109(): S109 { S109 { s: mk108() } }
    fun mk110(): S110 { S110 { s: mk109() } }
    fun mk111(): S111 { S111 { s: mk110() } }
    fun mk112(): S112 { S112 { s: mk111() } }
    fun mk113(): S113 { S113 { s: mk112() } }
    fun mk114(): S114 { S114 { s: mk113() } }
    fun mk115(): S115 { S115 { s: mk114() } }
    fun mk116(): S116 { S116 { s: mk115() } }
    fun mk117(): S117 { S117 { s: mk116() } }
    fun mk118(): S118 { S118 { s: mk117() } }
    fun mk119(): S119 { S119 { s: mk118() } }
    fun mk120(): S120 { S120 { s: mk119() } }
    fun mk121(): S121 { S121 { s: mk120() } }
    fun mk122(): S122 { S122 { s: mk121() } }
    fun mk123(): S123 { S123 { s: mk122() } }
    fun mk124(): S124 { S124 { s: mk123() } }
    fun mk125(): S125 { S125 { s: mk124() } }
    fun mk126(): S126 { S126 { s: mk125() } }
    fun mk127(): S127 { S127 { s: mk126() } }
    fun get0(value: &S0): u64 { value.x }
    fun get1(value: &S1): u64 { get0(&value.s) }
    fun get2(value: &S2): u64 { get1(&value.s) }
    fun get3(value: &S3): u64 { get2(&value.s) }
    fun get4(value: &S4): u64 { get3(&value.s) }
    fun get5(value: &S5): u64 { get4(&value.s) }
    fun get6(value: &S6): u64 { get5(&value.s) }
    fun get7(value: &S7): u64 { get6(&value.s) }
    fun get8(value: &S8): u64 { get7(&value.s) }
    fun get9(value: &S9): u64 { get8(&value.s) }
    fun get10(value: &S10): u64 { get9(&value.s) }
    fun get11(value: &S11): u64 { get10(&value.s) }
    fun get12(value: &S12): u64 { get11(&value.s) }
    fun get13(value: &S13): u64 { get12(&value.s) }
    fun get14(value: &S14): u64 { get13(&value.s) }
    fun get15(value: &S15): u64 { get14(&value.s) }
    fun get16(value: &S16): u64 { get15(&value.s) }
    fun get17(value: &S17): u64 { get16(&value.s) }
    fun get18(value: &S18): u64 { get17(&value.s) }
    fun get19(value: &S19): u64 { get18(&value.s) }
    fun get20(value: &S20): u64 { get19(&value.s) }
    fun get21(value: &S21): u64 { get20(&value.s) }
    fun get22(value: &S22): u64 { get21(&value.s) }
    fun get23(value: &S23): u64 { get22(&value.s) }
    fun get24(value: &S24): u64 { get23(&value.s) }
    fun get25(value: &S25): u64 { get24(&value.s) }
    fun get26(value: &S26): u64 { get25(&value.s) }
    fun get27(value: &S27): u64 { get26(&value.s) }
    fun get28(value: &S28): u64 { get27(&value.s) }
    fun get29(value: &S29): u64 { get28(&value.s) }
    fun get30(value: &S30): u64 { get29(&value.s) }
    fun get31(value: &S31): u64 { get30(&value.s) }
    fun get32(value: &S32): u64 { get31(&value.s) }
    fun get33(value: &S33): u64 { get32(&value.s) }
    fun get34(value: &S34): u64 { get33(&value.s) }
    fun get35(value: &S35): u64 { get34(&value.s) }
    fun get36(value: &S36): u64 { get35(&value.s) }
    fun get37(value: &S37): u64 { get36(&value.s) }
    fun get38(value: &S38): u64 { get37(&value.s) }
    fun get39(value: &S39): u64 { get38(&value.s) }
    fun get40(value: &S40): u64 { get39(&value.s) }
    fun get41(value: &S41): u64 { get40(&value.s) }
    fun get42(value: &S42): u64 { get41(&value.s) }
    fun get43(value: &S43): u64 { get42(&value.s) }
    fun get44(value: &S44): u64 { get43(&value.s) }
    fun get45(value: &S45): u64 { get44(&value.s) }
    fun get46(value: &S46): u64 { get45(&value.s) }
    fun get47(value: &S47): u64 { get46(&value.s) }
    fun get48(value: &S48): u64 { get47(&value.s) }
    fun get49(value: &S49): u64 { get48(&value.s) }
    fun get50(value: &S50): u64 { get49(&value.s) }
    fun get51(value: &S51): u64 { get50(&value.s) }
    fun get52(value: &S52): u64 { get51(&value.s) }
    fun get53(value: &S53): u64 { get52(&value.s) }
    fun get54(value: &S54): u64 { get53(&value.s) }
    fun get55(value: &S55): u64 { get54(&value.s) }
    fun get56(value: &S56): u64 { get55(&value.s) }
    fun get57(value: &S57): u64 { get56(&value.s) }
    fun get58(value: &S58): u64 { get57(&value.s) }
    fun get59(value: &S59): u64 { get58(&value.s) }
    fun get60(value: &S60): u64 { get59(&value.s) }
    fun get61(value: &S61): u64 { get60(&value.s) }
    fun get62(value: &S62): u64 { get61(&value.s) }
    fun get63(value: &S63): u64 { get62(&value.s) }
    fun get64(value: &S64): u64 { get63(&value.s) }
    fun get65(value: &S65): u64 { get64(&value.s) }
    fun get66(value: &S66): u64 { get65(&value.s) }
    fun get67(value: &S67): u64 { get66(&value.s) }
    fun get68(value: &S68): u64 { get67(&value.s) }
    fun get69(value: &S69): u64 { get68(&value.s) }
    fun get70(value: &S70): u64 { get69(&value.s) }
    fun get71(value: &S71): u64 { get70(&value.s) }
    fun get72(value: &S72): u64 { get71(&value.s) }
    fun get73(value: &S73): u64 { get72(&value.s) }
    fun get74(value: &S74): u64 { get73(&value.s) }
    fun get75(value: &S75): u64 { get74(&value.s) }
    fun get76(value: &S76): u64 { get75(&value.s) }
    fun get77(value: &S77): u64 { get76(&value.s) }
    fun get78(value: &S78): u64 { get77(&value.s) }
    fun get79(value: &S79): u64 { get78(&value.s) }
    fun get80(value: &S80): u64 { get79(&value.s) }
    fun get81(value: &S81): u64 { get80(&value.s) }
    fun get82(value: &S82): u64 { get81(&value.s) }
    fun get83(value: &S83): u64 { get82(&value.s) }
    fun get84(value: &S84): u64 { get83(&value.s) }
    fun get85(value: &S85): u64 { get84(&value.s) }
    fun get86(value: &S86): u64 { get85(&value.s) }
    fun get87(value: &S87): u64 { get86(&value.s) }
    fun get88(value: &S88): u64 { get87(&value.s) }
    fun get89(value: &S89): u64 { get88(&value.s) }
    fun get90(value: &S90): u64 { get89(&value.s) }
    fun get91(value: &S91): u64 { get90(&value.s) }
    fun get92(value: &S92): u64 { get91(&value.s) }
    fun get93(value: &S93): u64 { get92(&value.s) }
    fun get94(value: &S94): u64 { get93(&value.s) }
    fun get95(value: &S95): u64 { get94(&value.s) }
    fun get96(value: &S96): u64 { get95(&value.s) }
    fun get97(value: &S97): u64 { get96(&value.s) }
    fun get98(value: &S98): u64 { get97(&value.s) }
    fun get99(value: &S99): u64 { get98(&value.s) }
    fun get100(value: &S100): u64 { get99(&value.s) }
    fun get101(value: &S101): u64 { get100(&value.s) }
    fun get102(value: &S102): u64 { get101(&value.s) }
    fun get103(value: &S103): u64 { get102(&value.s) }
    fun get104(value: &S104): u64 { get103(&value.s) }
    fun get105(value: &S105): u64 { get104(&value.s) }
    fun get106(value: &S106): u64 { get105(&value.s) }
    fun get107(value: &S107): u64 { get106(&value.s) }
    fun get108(value: &S108): u64 { get107(&value.s) }
    fun get109(value: &S109): u64 { get108(&value.s) }
    fun get110(value: &S110): u64 { get109(&value.s) }
    fun get111(value: &S111): u64 { get110(&value.s) }
    fun get112(value: &S112): u64 { get111(&value.s) }
    fun get113(value: &S113): u64 { get112(&value.s) }
    fun get114(value: &S114): u64 { get113(&value.s) }
    fun get115(value: &S115): u64 { get114(&value.s) }
    fun get116(value: &S116): u64 { get115(&value.s) }
    fun get117(value: &S117): u64 { get116(&value.s) }
    fun get118(value: &S118): u64 { get117(&value.s) }
    fun get119(value: &S119): u64 { get118(&value.s) }
    fun get120(value: &S120): u64 { get119(&value.s) }
    fun get121(value: &S121): u64 { get120(&value.s) }
    fun get122(value: &S122): u64 { get121(&value.s) }
    fun get123(value: &S123): u64 { get122(&value.s) }
    fun get124(value: &S124): u64 { get123(&value.s) }
    fun get125(value: &S125): u64 { get124(&value.s) }
    fun get126(value: &S126): u64 { get125(&value.s) }
    fun get127(value: &S127): u64 { get126(&value.s) }
    public fun value_depth(): u64 {
        let original = mk126();
        let copied = original;
        let same = if (original == copied) 1000 else 0;
        same + get126(&copied) + vector::length(&bcs::to_bytes(&original))
    }
    public fun value_depth_129(): u64 {
        let original = mk127();
        let copied = original;
        let same = if (original == copied) 1000 else 0;
        same + get127(&copied) + vector::length(&bcs::to_bytes(&original))
    }
    fun l0<T: drop>(): u64 { 1 + l1<vector<T>>() }
    fun l1<T: drop>(): u64 { 1 + l2<vector<T>>() }
    fun l2<T: drop>(): u64 { 1 + l3<vector<T>>() }
    fun l3<T: drop>(): u64 { 1 + l4<vector<T>>() }
    fun l4<T: drop>(): u64 { 1 + l5<vector<T>>() }
    fun l5<T: drop>(): u64 { 1 + l6<vector<T>>() }
    fun l6<T: drop>(): u64 { 1 + l7<vector<T>>() }
    fun l7<T: drop>(): u64 { 1 + l8<vector<T>>() }
    fun l8<T: drop>(): u64 { 1 + l9<vector<T>>() }
    fun l9<T: drop>(): u64 { 1 + l10<vector<T>>() }
    fun l10<T: drop>(): u64 { 1 + l11<vector<T>>() }
    fun l11<T: drop>(): u64 { 1 + l12<vector<T>>() }
    fun l12<T: drop>(): u64 { 1 + l13<vector<T>>() }
    fun l13<T: drop>(): u64 { 1 + l14<vector<T>>() }
    fun l14<T: drop>(): u64 { 1 + l15<vector<T>>() }
    fun l15<T: drop>(): u64 { 1 + l16<vector<T>>() }
    fun l16<T: drop>(): u64 { 1 + l17<vector<T>>() }
    fun l17<T: drop>(): u64 { 1 + l18<vector<T>>() }
    fun l18<T: drop>(): u64 { 1 + l19<vector<T>>() }
    fun l19<T: drop>(): u64 { 0 }
    struct P<A, B> has drop { a: A, b: B }
    fun p0<T: drop>(extra: bool): u64 { 1 + p1<P<T, T>>(extra) }
    fun p1<T: drop>(extra: bool): u64 { 1 + p2<P<T, T>>(extra) }
    fun p2<T: drop>(extra: bool): u64 { 1 + p3<P<T, T>>(extra) }
    fun p3<T: drop>(extra: bool): u64 { 1 + p4<P<T, T>>(extra) }
    fun p4<T: drop>(extra: bool): u64 { 1 + p5<P<T, T>>(extra) }
    fun p5<T: drop>(extra: bool): u64 { 1 + p6<P<T, T>>(extra) }
    fun p6<T: drop>(extra: bool): u64 {
        if (extra) stop<vector<vector<T>>>() else stop<vector<T>>()
    }
    fun stop<T: drop>(): u64 { 100 }
    public fun type_depth(): u64 { l0<u8>() }
    public fun type_depth_21(): u64 { l0<vector<u8>>() }
    public fun type_nodes_128(): u64 { p0<u8>(false) }
    public fun type_nodes_129(): u64 { p0<u8>(true) }
}

// RUN: execute 0x42::deep::value_depth
// CHECK: results: 1015

// RUN: execute 0x42::deep::value_depth_129
// CHECK-V1-SUBSTR: VM_MAX_VALUE_DEPTH_REACHED
// CHECK-V2: results: 1015

// RUN: execute 0x42::deep::type_depth
// CHECK: results: 19

// RUN: execute 0x42::deep::type_depth_21
// CHECK-V1-SUBSTR: VM_MAX_TYPE_DEPTH_REACHED
// CHECK-V2: results: 19

// RUN: execute 0x42::deep::type_nodes_128
// CHECK: results: 106

// RUN: execute 0x42::deep::type_nodes_129
// CHECK-V1-SUBSTR: TOO_MANY_TYPE_NODES
// CHECK-V2: results: 106
