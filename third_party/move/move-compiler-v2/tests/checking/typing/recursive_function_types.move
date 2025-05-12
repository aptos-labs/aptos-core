module 0xc0ffed::m {
    struct S(|S|) has copy, drop;
}

module 0xc0ffee::m1 {
    struct S1(|S2|) has copy, drop;
    struct S2(|S1|) has copy, drop;
}

module 0xc0ffef::m2 {
    struct S1(|S2|) has copy, drop;
    struct S2(|S3|) has copy, drop;
    struct S3(|S1|) has copy, drop;
}

module 0xc0fff0::m3 {
    struct S1(|S2|) has copy, drop;
    struct S2(|S3|) has copy, drop;
    struct S3(|S4|) has copy, drop;
    struct S4(|S1|) has copy, drop;
}

module 0xc0fff1::m4 {
    struct S1(||S1) has copy, drop;
}

module 0xc0fff2::m5 {
    struct S(||S) has copy, drop;
}

module 0xc0fff3::m6 {
    struct S1(|S2|) has copy, drop;
    struct S2(||S1) has copy, drop;
}

module 0xc0fff4::m7 {
    struct S1(|S2|) has copy, drop;
    struct S2(|S3|) has copy, drop;
    struct S3(||S1) has copy, drop;
}

module 0xc0fff5::m8 {
    struct S1(|S2|) has copy, drop;
    struct S2(|S3|) has copy, drop;
    struct S3(|S4|) has copy, drop;
    struct S4(||S1) has copy, drop;
}


module 0xc0fff6::m9 {
    struct S1(|S2|) has copy, drop;
    struct S2(|S3|) has copy, drop;
    struct S3(|S4|) has copy, drop;
    struct S4(||S1) has copy, drop;
}

module 0xc0fff7::m10 {
    struct S1  { x: u64, y: u64, f : S2 } has copy, drop;
    struct S2(|S1|) has copy, drop;
}

module 0xc0fff8::m11 {
    struct S1  { x: u64, y: u64, f : S2 } has copy, drop;
    struct S2(|S3|) has copy, drop;
    struct S3(|S1|) has copy, drop;
}

module 0xc0fff9::m12 {
    struct S1  { x: u64, y: u64, f : S2 } has copy, drop;
    struct S2(|S3|) has copy, drop;
    struct S3(|S4|) has copy, drop;
    struct S4(|S1|) has copy, drop;
}

module 0xc0fff10::m13 {
    struct S1  { x: u64, y: u64, f : S2 } has copy, drop;
    struct S2(|&S1|) has copy, drop;
}

module 0xc0fff11::m14 {
    struct S1  { x: u64, y: u64, f : S2 } has copy, drop;
    struct S2(|S3|) has copy, drop;
    struct S3(|&S1|) has copy, drop;
}

module 0xc0fff12::m15 {
    struct S1  { x: u64, y: u64, f : S2 } has copy, drop;
    struct S2(|S3|) has copy, drop;
    struct S3(|S4|) has copy, drop;
    struct S4(|&S1|) has copy, drop;
}

module 0xc0fff13::m16 {
    struct S1 { x: u64, y: u64, f : S2 } has copy, drop;
    enum E {
        A{s: S1},
        B{a: u64}
    } has copy, drop;
    struct S2(|E|) has copy, drop;
}

module 0xc0fff14::m17 {
    struct S1 { x: u64, y: u64, f : S2 } has copy, drop;
    enum E {
        A{s: S1},
        B{a: u64}
    } has copy, drop;
    struct S2(|&E|) has copy, drop;
}

module 0xc0fff15::m18 {
    struct S1 { x: u64, y: u64, f : S2 } has copy, drop;
    enum E {
        A{s: S1},
        B{a: u64}
    } has copy, drop;
    struct S2(|S3|) has copy, drop;
    struct S3(|&E|) has copy, drop;
}

module 0xc0fff16::m19 {
    struct S1 { x: u64, y: u64, f : S2 } has copy, drop;
    enum E {
        A{s: S1},
        B{a: u64}
    } has copy, drop;
    struct S2(|S3|) has copy, drop;
    struct S3(|S4|) has copy, drop;
    struct S4(|&E|) has copy, drop;
}
