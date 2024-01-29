module TestFunFormat {

    struct SomeOtherStruct has drop {
        some_field: u64,
    }

    // test case: many blank lines between functions.
    public fun fun1(v: u64): SomeOtherStruct {
        SomeOtherStruct {some_field: v}
    }

    public fun fun2(v: u64): SomeOtherStruct {
        SomeOtherStruct {some_field: v}
    }

}