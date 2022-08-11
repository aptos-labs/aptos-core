module aptos_std::type_map {
    use aptos_std::simple_map::{Self, SimpleMap};
    use aptos_std::type_info::{Self, TypeInfo};
    use aptos_std::any;

    use std::bcs;

    struct TypeMap has drop, copy, store {
        inner: SimpleMap<TypeInfo, vector<u8>>
    }

    public fun create(): TypeMap {
        TypeMap { inner: simple_map::create() }
    }

    public fun contains<T>(map: &TypeMap): bool {
        let ty_info = type_info::type_of<T>();
        simple_map::contains_key(&map.inner, &ty_info)
    }

    public fun move_in<T: drop>(map: &mut TypeMap, value: T) {
        let ty_info = type_info::type_of<T>();
        let value_bcs = bcs::to_bytes(&value);
        simple_map::add(&mut map.inner, ty_info, value_bcs);
    }

    public fun clone<T: copy>(map: &mut TypeMap): T {
        let ty_info = type_info::type_of<T>();
        let bcs = *simple_map::borrow_mut(&mut map.inner, &ty_info);
        any::from_bytes<T>(bcs)
    }

    public fun move_out<T>(map: &mut TypeMap): T {
        let ty_info = type_info::type_of<T>();
        let (_, bcs) = simple_map::remove(&mut map.inner, &ty_info);
        any::from_bytes<T>(bcs)
    }
}
