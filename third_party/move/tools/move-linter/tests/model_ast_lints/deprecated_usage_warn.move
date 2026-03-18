module 0xc0ffee::dep {
    #[deprecated]
    public fun old_fun(): u64 { 42 }

    public fun new_fun(): u64 { 42 }

    #[deprecated]
    struct OldStruct has copy, drop {
        value: u64,
    }

    struct NewStruct has copy, drop {
        value: u64,
    }

    // Should warn: non-deprecated function using deprecated struct (pack)
    public fun make_old(): OldStruct {
        OldStruct { value: 1 }
    }

    // Should not warn: non-deprecated function using non-deprecated struct
    public fun make_new(): NewStruct {
        NewStruct { value: 1 }
    }

    // Should not warn: deprecated function using deprecated struct
    #[deprecated]
    public fun old_make_old(): OldStruct {
        OldStruct { value: 1 }
    }

    // Should warn: non-deprecated function destructuring deprecated struct
    public fun unpack_old(s: OldStruct): u64 {
        let OldStruct { value } = s;
        value
    }
}

module 0xc0ffee::user {
    use 0xc0ffee::dep;

    // Should warn: calling deprecated function
    public fun calls_deprecated(): u64 {
        dep::old_fun()
    }

    // Should not warn: calling non-deprecated function
    public fun calls_new(): u64 {
        dep::new_fun()
    }
}

// Module-level deprecation
#[deprecated]
module 0xc0ffee::old_mod {
    public fun still_works(): u64 { 1 }
}

module 0xc0ffee::user2 {
    use 0xc0ffee::old_mod;

    // Should warn: calling function in deprecated module
    public fun uses_old_mod(): u64 {
        old_mod::still_works()
    }
}

// Should not warn: deprecated module using deprecated items
#[deprecated]
module 0xc0ffee::also_deprecated {
    use 0xc0ffee::dep;
    use 0xc0ffee::old_mod;

    public fun uses_both(): u64 {
        dep::old_fun() + old_mod::still_works()
    }
}

// Test suppression with lint::skip
module 0xc0ffee::suppressed {
    use 0xc0ffee::dep;

    #[lint::skip(deprecated_usage)]
    public fun suppressed_call(): u64 {
        dep::old_fun()
    }
}

// Test field type checking: struct with deprecated type in field
module 0xc0ffee::field_check {
    use 0xc0ffee::dep;

    // Should warn: field type references deprecated struct
    struct Holder has copy, drop {
        inner: dep::OldStruct,
    }

    // Should not warn: field type references non-deprecated struct
    struct GoodHolder has copy, drop {
        inner: dep::NewStruct,
    }
}

// Test function signature checking
module 0xc0ffee::sig_check {
    use 0xc0ffee::dep;

    // Should warn: parameter type references deprecated struct
    public fun takes_old(_s: dep::OldStruct): u64 { 0 }

    // Should warn: return type references deprecated struct
    public fun returns_old(): dep::OldStruct { dep::make_old() }

    // Should not warn: non-deprecated types
    // (also tests that unused import of deprecated module does NOT trigger
    //  from this module since it's used by functions above)
    public fun takes_new(_s: dep::NewStruct): u64 { 0 }
}

// Test use-declaration checking: importing deprecated module
module 0xc0ffee::use_check {
    // Should warn: importing a deprecated module
    use 0xc0ffee::old_mod;

    public fun f(): u64 { old_mod::still_works() }
}

// Test use-declaration checking: importing deprecated member
module 0xc0ffee::use_member_check {
    // Should warn: importing a deprecated function by name
    use 0xc0ffee::dep::old_fun;

    public fun f(): u64 { old_fun() }
}

// Test deprecated constant usage (within same module, since constants are private)
module 0xc0ffee::const_check {
    #[deprecated]
    const OLD_CONST: u64 = 99;
    const NEW_CONST: u64 = 100;

    // Should warn: using deprecated constant
    public fun uses_old_const(): u64 { OLD_CONST }

    // Should not warn
    public fun uses_new_const(): u64 { NEW_CONST }
}
