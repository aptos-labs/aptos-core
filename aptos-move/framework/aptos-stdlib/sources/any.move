module aptos_std::any {
    use aptos_std::type_info::{Self, TypeInfo};
    use std::bcs;
    use std::error;

    friend aptos_std::type_map;

    const ETYPE_MISMATCH: u64 = 0;

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
    ///
    /// TODO: currently this is restricted to structs, we may want to relax this. Restriction comes from TypeInfo.
    struct Any has drop, store {
        type_info: TypeInfo,
        data: vector<u8>
    }

    /// Pack a value into the `Any` representation. Because Any can be stored and dropped, this is
    /// also required from `T`.
    public fun pack<T: drop + store>(x: T): Any {
        Any {
            type_info: type_info::type_of<T>(),
            data: bcs::to_bytes(&x)
        }
    }

    /// Unpack a value from the `Any` representation. This aborts if the value has not the expected type `T`.
    public fun unpack<T>(x: Any): T {
        assert!(type_info::type_of<T>() == x.type_info, error::invalid_argument(ETYPE_MISMATCH));
        from_bytes<T>(x.data)
    }

    /// Returns the type info of this Any
    public fun type_info(x: &Any): TypeInfo {
        x.type_info
    }

    /// Native function to deserialize a type T.
    ///
    /// Note that this function does not put any constraint on `T`. If code uses this function to
    /// deserialized a linear value, its their responsibility that the data they deserialize is
    /// owned.
    public(friend) native fun from_bytes<T>(bytes: vector<u8>): T;
}
