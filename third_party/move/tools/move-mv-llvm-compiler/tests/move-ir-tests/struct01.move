
module 0x100::M {

    struct MyStruct {
        field1: u32,
        field2: bool,
        field3: EmptyStruct
    }

    struct EmptyStruct {}

    public fun boofun(): 0x100::M::MyStruct {
        MyStruct { field1: 32, field2: true, field3: EmptyStruct {} }
    }
}
