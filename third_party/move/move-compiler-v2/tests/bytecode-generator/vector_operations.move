module 0x42::vector_ops {
    use std::vector;

    struct Container has drop {
        items: vector<u64>,
        count: u64,
    }

    // Test basic vector operations
    fun sum_vector(v: vector<u64>): u64 {
        let sum = 0;
        let len = vector::length(&v);
        let i = 0;
        while (i < len) {
            sum = sum + *vector::borrow(&v, i);
            i = i + 1;
        };
        sum
    }

    // Test vector with conditional logic
    fun filter_greater_than(v: vector<u64>, threshold: u64): vector<u64> {
        let result = vector::empty();
        let len = vector::length(&v);
        let i = 0;
        while (i < len) {
            let val = *vector::borrow(&v, i);
            if (val > threshold) {
                vector::push_back(&mut result, val);
            };
            i = i + 1;
        };
        result
    }

    // Test vector with early return
    fun find_first_greater(v: vector<u64>, threshold: u64): u64 {
        let len = vector::length(&v);
        let i = 0;
        while (i < len) {
            let val = *vector::borrow(&v, i);
            if (val > threshold) {
                return val
            };
            i = i + 1;
        };
        0
    }

    // Test vector operations with struct
    fun create_container(items: vector<u64>): Container {
        let count = vector::length(&items);
        Container { items, count }
    }

    // Test vector mutation through struct
    fun add_item(container: &mut Container, item: u64) {
        vector::push_back(&mut container.items, item);
        container.count = container.count + 1;
    }

    // Test vector with nested control flow
    fun count_in_range(v: vector<u64>, min: u64, max: u64): u64 {
        let count = 0;
        let len = vector::length(&v);
        let i = 0;
        while (i < len) {
            let val = *vector::borrow(&v, i);
            if (val >= min) {
                if (val <= max) {
                    count = count + 1;
                };
            };
            i = i + 1;
        };
        count
    }

    // Test vector with break
    fun has_value(v: vector<u64>, target: u64): bool {
        let len = vector::length(&v);
        let i = 0;
        while (i < len) {
            if (*vector::borrow(&v, i) == target) {
                return true
            };
            i = i + 1;
        };
        false
    }
}
