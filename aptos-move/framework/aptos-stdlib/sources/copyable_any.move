module aptos_std::copyable_any {
    use aptos_std::type_info;
    use aptos_std::from_bcs::from_bytes;
    use std::bcs;
    use std::error;
    use std::string::String;

    /// The type provided for `unpack` is not the same as was given for `pack`.
    const ETYPE_MISMATCH: u64 = 0;

    /// The same as `any::Any` but with the copy ability.
    struct Any has drop, store, copy {
        type_name: String,
        data: vector<u8>
    }

    /// Pack a value into the `Any` representation. Because Any can be stored, dropped, and copied this is
    /// also required from `T`.
    public fun pack<T: drop + store + copy>(x: T): Any {
        Any {
            type_name: type_info::type_name<T>(),
            data: bcs::to_bytes(&x)
        }
    }

    /// Unpack a value from the `Any` representation. This aborts if the value has not the expected type `T`.
    public fun unpack<T>(self: Any): T {
        assert!(type_info::type_name<T>() == self.type_name, error::invalid_argument(ETYPE_MISMATCH));
        from_bytes<T>(self.data)
    }

    /// Returns the type name of this Any
    public fun type_name(self: &Any): &String {
        &self.type_name
    }

    #[test_only]
    struct S has store, drop, copy { x: u64 }

    #[test]
    fun test_any() {
        assert!(pack(22).unpack::<u64>() == 22, 1);
        assert!(pack(S { x: 22 }).unpack::<S>() == S { x: 22 }, 2);
    }
}
