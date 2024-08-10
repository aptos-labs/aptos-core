module aptos_std::any_map {
    use std::bcs::to_bytes;
    use std::option;
    use std::string::String;
    use std::vector;
    use aptos_std::from_bcs;
    // TODO: change to ordered_map
    use aptos_std::simple_map::{Self, SimpleMap};
    use aptos_std::type_info;

    /// The type provided for `unpack` is not the same as was given for `pack`.
    const ETYPE_MISMATCH: u64 = 1;

    struct AnyMap has drop, store {
        entries: SimpleMap<String, vector<u8>>,
    }

    public fun new(): AnyMap {
        AnyMap {
            entries: simple_map::new(),
        }
    }

    public fun add<T: drop + store>(self: &mut AnyMap, x: T) {
        simple_map::add(&mut self.entries, type_info::type_name<T>(), to_bytes(&x));
    }

    public fun get_copy<T: copy + drop + store>(self: &AnyMap): T {
        let data = simple_map::borrow(&self.entries, &type_info::type_name<T>());
        from_bcs::from_bytes<T>(vector::slice(data, 0, vector::length(data)))
    }

    public fun remove<T>(self: &mut AnyMap): T {
        let (_key, data) = simple_map::remove(&mut self.entries, &type_info::type_name<T>());
        from_bcs::from_bytes<T>(data)
    }

    public fun remove_if_present<T>(self: &mut AnyMap): option::Option<T> {
        let data = simple_map::remove_if_present(&mut self.entries, &type_info::type_name<T>());
        if (option::is_some(&data)) {
            option::some(from_bcs::from_bytes<T>(option::destroy_some(data)))
        } else {
            option::none()
        }
    }

    public fun length(self: &AnyMap): u64 {
        simple_map::length(&self.entries)
    }

    public fun to_raw_vec_pair(self: AnyMap): (vector<String>, vector<vector<u8>>) {
        let AnyMap { entries } = self;
        simple_map::to_vec_pair(entries)
    }
}
