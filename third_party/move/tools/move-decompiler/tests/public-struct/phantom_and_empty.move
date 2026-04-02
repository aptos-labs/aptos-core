// Tests decompilation of public structs with phantom type parameters and
// empty structs (no fields). These are corner cases for the pack/unpack API:
//   - Phantom types: instantiation carries type argument but no runtime field
//   - Empty struct: pack/unpack with zero fields
// Based on compiler-v2 tests: struct_phantoms.move, struct_pack_unpack_api.move

module 0x42::phantom_defs {
    /// A struct with a phantom type parameter: no field carries T at runtime.
    public struct Handle<phantom T> has copy, drop {
        id: u64,
    }

    /// A struct with both a phantom and a real type parameter.
    public struct Tagged<phantom K, V: copy + drop> has copy, drop {
        value: V,
    }

    /// A struct that carries no fields whatsoever.
    public struct Empty has copy, drop {}

    /// A struct with a phantom that also holds a capability marker.
    public struct Cap<phantom T> has copy, drop {
        addr: address,
    }
}

module 0x42::phantom_consumer {
    use 0x42::phantom_defs::{Handle, Tagged, Empty, Cap};

    // -----------------------------------------------------------------------
    // Handle<phantom T>
    // -----------------------------------------------------------------------

    fun make_handle<T>(id: u64): Handle<T> {
        Handle { id }
    }

    fun get_id<T>(h: &Handle<T>): u64 {
        *&h.id
    }

    fun unpack_handle<T>(h: Handle<T>): u64 {
        let Handle { id } = h;
        id
    }

    // Two different instantiations of the same phantom struct.
    fun use_two_handles(): u64 {
        let h1 = make_handle<u8>(1);
        let h2 = make_handle<u64>(2);
        get_id(&h1) + get_id(&h2)
    }

    // -----------------------------------------------------------------------
    // Tagged<phantom K, V>
    // -----------------------------------------------------------------------

    fun make_tagged<K, V: copy + drop>(value: V): Tagged<K, V> {
        Tagged { value }
    }

    fun get_value<K, V: copy + drop>(t: &Tagged<K, V>): V {
        *&t.value
    }

    fun unpack_tagged<K, V: copy + drop>(t: Tagged<K, V>): V {
        let Tagged { value } = t;
        value
    }

    // -----------------------------------------------------------------------
    // Empty struct
    // -----------------------------------------------------------------------

    fun make_empty(): Empty {
        Empty {}
    }

    fun consume_empty(e: Empty) {
        let Empty {} = e;
    }

    fun roundtrip_empty() {
        let e = make_empty();
        consume_empty(e);
    }

    // -----------------------------------------------------------------------
    // Cap<phantom T>
    // -----------------------------------------------------------------------

    fun make_cap<T>(addr: address): Cap<T> {
        Cap { addr }
    }

    fun get_cap_addr<T>(c: &Cap<T>): address {
        *&c.addr
    }

    // -----------------------------------------------------------------------
    // End-to-end
    // -----------------------------------------------------------------------

    fun end_to_end(): u64 {
        let h = make_handle<bool>(10);
        let id = unpack_handle(h);
        let t = make_tagged<u8, u64>(20);
        let v = unpack_tagged<u8, u64>(t);
        roundtrip_empty();
        id + v
    }
}
