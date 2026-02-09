module 0x42::types {
    // These structs should not be marked as unused because they're public
    // and could be used by other modules
    public struct PublicType1 has drop {
        x: u64
    }

    public struct PublicType2 has drop {
        y: u64
    }

    // Private struct unused in this module - should warn
    struct PrivateUnused {
        z: u64
    }

    // Private struct used internally - should not warn
    struct PrivateUsed {
        w: u64
    }

    public fun make_public_type1(x: u64): PublicType1 {
        PublicType1 { x }
    }

    public fun use_private(): PrivateUsed {
        PrivateUsed { w: 1 }
    }
}

module 0x42::consumer {
    use 0x42::types;

    // This struct uses a type from another module
    struct UsesImportedType has drop {
        field: types::PublicType1
    }

    public fun test(): UsesImportedType {
        UsesImportedType {
            field: types::make_public_type1(1)
        }
    }

    // Use PublicType2 in function parameter
    public fun consume_type2(_x: types::PublicType2) {
    }
}
