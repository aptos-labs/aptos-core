module publisher::test {
    use std::vector;

    public entry fun just_under_quota() {
        let v = vector::empty();
        let t = vector::empty();
        let i = 0;
        while (i < 1000) {
            vector::push_back(&mut v, 0u128);
            i = i + 1;
        };

        i = 0;
        while (i < 622) {
            vector::push_back(&mut t, copy v);
            i = i + 1;
        }
    }

    public entry fun just_above_quota() {
        let v = vector::empty();
        let t = vector::empty();
        let i = 0;
        while (i < 1000) {
            vector::push_back(&mut v, 0u128);
            i = i + 1;
        };

        i = 0;
        while (i < 623) {
            vector::push_back(&mut t, copy v);
            i = i + 1;
        }
    }
}
