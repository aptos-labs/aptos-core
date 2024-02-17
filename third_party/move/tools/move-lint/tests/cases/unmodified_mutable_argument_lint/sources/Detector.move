module NamedAddr::Detector {
    struct MyStruct has key {
        value: u64,
    }

    public fun function_do_not_modify(my_struct: &mut MyStruct): u64 {
        my_struct.value
    }

    public fun function_modify_mut_arg(my_struct: &mut MyStruct) {
        my_struct.value = 42;
    }

}
