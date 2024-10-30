module publisher::test {
    use std::vector;
    use aptos_std::table_with_length as table;

    public entry fun just_under_quota() {
        let v = vector::empty();
        let i = 0;
        while (i < 1000) {
            vector::push_back(&mut v, 0u128);
            i = i + 1;
        };

        let t = table::new();

        i = 0;
        while (i < 622) {
            table::add(&mut t, i, copy v);
            i = i + 1;
        };

        table::destroy_empty(t);
    }

    public entry fun just_above_quota() {
        let v = vector::empty();
        let i = 0;
        while (i < 1000) {
            vector::push_back(&mut v, 0u128);
            i = i + 1;
        };

        let t = table::new();

        i = 0;
        while (i < 623) {
            table::add(&mut t, i, copy v);
            i = i + 1;
        };

        table::destroy_empty(t);
    }
}
