//# publish
module 0x42::m {

    enum MyEnum has drop, copy {
        V { x: u64 }
    }

    struct MyStruct has drop, copy {
        x: u64,
    }

    fun test_struct_block_mutation(): u64 {
        let s = MyStruct { x: 10 };
        *(&mut ({ s }).x) = 42;
        s.x
    }

    fun test_enum_block_mutation(): u64 {
        let e = MyEnum::V { x: 10 };
        *(&mut ({ e }).x) = 42; // modifies a temp value
        e.x
    }
}

//# run 0x42::m::test_struct_block_mutation

//# run 0x42::m::test_enum_block_mutation
