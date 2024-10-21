module aptos_std::hot_potato_any {
    use aptos_std::type_info;
    use aptos_std::from_bcs::from_bytes;
    use std::bcs::to_bytes;
    use std::error;
    use std::string::String;

    /// The type provided for `unpack` is not the same as was given for `pack`.
    const ETYPE_MISMATCH: u64 = 1;

    /// A type which can represent a value of any type. This allows for representation of 'unknown' future
    /// values. For example, to define a resource such that it can be later be extended without breaking
    /// changes one can do
    ///
    /// ```move
    ///   struct Resource {
    ///      field: Type,
    ///      ...
    ///      extension: Option<Any>
    ///   }
    /// ```
    struct Any {
        type_name: String,
        data: vector<u8>
    }

    /// Pack a value into the `Any` representation. x is dropped using native but it's fine because Any itself has no abilities.
    public fun pack<T>(x: T): Any {
        let data = to_bytes(&x);
        drop(x);
        Any {
            type_name: type_info::type_name<T>(),
            data,
        }
    }

    /// Unpack a value from the `Any` representation. This aborts if the value has not the expected type `T`.
    public fun unpack<T>(self: Any): T {
        let Any { type_name, data } = self;
        assert!(type_info::type_name<T>() == type_name, error::invalid_argument(ETYPE_MISMATCH));
        from_bytes<T>(data)
    }

    /// Returns the type name of this Any
    public fun type_name(self: &Any): &String {
        &self.type_name
    }

    // Dropping value.
    native fun drop<T>(x: T);

    #[test_only]
    struct S has store, drop { x: u64 }

    #[test]
    fun test_any() {
        assert!(unpack<u64>(pack(22)) == 22, 1);
        assert!(unpack<S>(pack(S { x: 22 })) == S { x: 22 }, 2);
    }
}
