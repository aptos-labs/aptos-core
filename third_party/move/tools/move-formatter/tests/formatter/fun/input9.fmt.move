// test case: multiple blank lines after module begins and before it ends.
module TestFunFormat {

    struct SomeOtherStruct has drop {
        some_field: u64,
    }

    public fun fun1(v: u64): SomeOtherStruct {
        SomeOtherStruct {some_field: v}
    }

    public fun fun2(v: u64): SomeOtherStruct {
        SomeOtherStruct {some_field: v}
    }

}