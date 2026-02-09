module 0x42::m {
    // Used as field type in another struct
    struct UsedAsFieldType {
        x: u64
    }

    struct Container {
        inner: UsedAsFieldType
    }

    // Used in tuple type
    struct UsedInTuple {
        y: u64
    }

    // Used in nested vector
    struct UsedInNestedVector {
        z: u64
    }

    // Used in generic type parameter
    struct Box<T> has drop {
        value: T
    }

    struct UsedInGeneric {
        a: u64
    }

    // Used in explicit type annotation
    struct UsedInTypeAnnotation {
        b: u64
    }

    // Used in cast/type hint
    struct UsedInCast has drop {
        c: u64
    }

    // Used as reference parameter (mutable)
    struct UsedAsMutRef {
        d: u64
    }

    // Used in option type
    struct UsedInOption has drop {
        e: u64
    }

    // Really unused struct
    struct TrulyUnused {
        f: u64
    }

    public fun use_as_field(): Container {
        Container { inner: UsedAsFieldType { x: 1 } }
    }

    public fun use_in_tuple(): (UsedInTuple, u64) {
        (UsedInTuple { y: 2 }, 3)
    }

    public fun use_in_nested_vector(): vector<vector<UsedInNestedVector>> {
        vector[]
    }

    public fun use_in_generic(): Box<UsedInGeneric> {
        Box { value: UsedInGeneric { a: 4 } }
    }

    public fun use_in_type_annotation() {
        let _x: UsedInTypeAnnotation = UsedInTypeAnnotation { b: 5 };
    }

    public fun use_in_cast(): UsedInCast {
        UsedInCast { c: 6 }
    }

    public fun use_as_mut_ref(s: &mut UsedAsMutRef) {
        s.d = 7;
    }

    public fun use_in_option(): std::option::Option<UsedInOption> {
        std::option::none()
    }
}
