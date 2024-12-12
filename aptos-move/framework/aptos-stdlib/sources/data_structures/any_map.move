module aptos_std::any_map {
    use std::bcs::to_bytes;
    use std::option;
    use std::string::String;
    use std::vector;
    use aptos_std::from_bcs;
    use aptos_std::ordered_map::{Self, OrderedMap};
    use aptos_std::type_info;

    /// The type provided for `unpack` is not the same as was given for `pack`.
    const ETYPE_MISMATCH: u64 = 1;

    struct AnyMap has drop, store {
        entries: OrderedMap<String, vector<u8>>,
    }

    public fun new(): AnyMap {
        AnyMap {
            entries: ordered_map::new(),
        }
    }

    public fun add<T: drop + store>(self: &mut AnyMap, x: T) {
        self.entries.add(type_info::type_name<T>(), to_bytes(&x));
    }

    public fun get_copy<T: copy + drop + store>(self: &AnyMap): T {
        let data = self.entries.borrow(&type_info::type_name<T>());
        from_bcs::from_bytes<T>(vector::slice(data, 0, vector::length(data)))
    }

    public fun remove<T>(self: &mut AnyMap): T {
        let data = self.entries.remove(&type_info::type_name<T>());
        from_bcs::from_bytes<T>(data)
    }

    public fun remove_if_present<T>(self: &mut AnyMap): option::Option<T> {
        let iter = self.entries.find(&type_info::type_name<T>());
        if (iter.iter_is_end(&self.entries)) {
            option::none()
        } else {
            option::some(from_bcs::from_bytes<T>(iter.iter_remove(&mut self.entries)))
        }
    }

    public fun length(self: &AnyMap): u64 {
        self.entries.length()
    }
}
