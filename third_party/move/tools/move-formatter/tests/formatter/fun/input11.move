module TestFunFormat {

    struct SomeOtherStruct has drop {
        some_field: u64,
    }

    public fun multi_arg(p1:u64,p2:u64,p3:u64,p4:u64,p5:u64,p6:u64,       p7:u64,p8:u64,p9:u64,p10:u64,p11:u64,p12:u64,p13:u64,p14:u64):u64{
        p1 + p2
    }
}
