module 0x42::vectors {
    use std::vector;

    fun push_pop(): u64 {
        let v = vector::empty<u64>();
        vector::push_back(&mut v, 1);
        vector::push_back(&mut v, 2);
        let x = vector::pop_back(&mut v);
        x + vector::length(&v)
    }

    fun contains_value(): bool {
        let v = vector[1, 2, 3];
        vector::contains(&v, &2)
    }

    fun swap_values(): u64 {
        let v = vector[1, 2, 3];
        vector::swap(&mut v, 0, 2);
        *vector::borrow(&v, 0)
    }

    fun set_middle(): u64 {
        let values = vector[10, 20, 30];
        let middle = vector::borrow_mut(&mut values, 1);
        *middle = 42;
        *vector::borrow(&values, 1)
    }

    fun sum(v: &vector<u64>): u64 {
        let total = 0;
        let i = 0;
        while (i < vector::length(v)) {
            total = total + *vector::borrow(v, i);
            i = i + 1;
        };
        total
    }

    fun bytes(): vector<u8> {
        b"Move"
    }
}
