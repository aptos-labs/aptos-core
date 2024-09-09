module 0x42::test {
    use std::option::{Self, Option};
    use std::vector;

    public fun check_and_get_threshold(bytes: vector<u8>): Option<u8> {
        let len = vector::length(&bytes);
        if (len == 0) {
            return option::none<u8>()
        };
        let x = len % 42;
        let y = len / 42;
        let bar = *vector::borrow(&bytes, len - 1);
        if (y == 0 || y > 42 || x != 1) {
            return option::none<u8>()
        } else if (bar == 0 || bar > (y as u8)) {
            return option::none<u8>()
        } else {
            return option::some(bar)
        }
    }
}
