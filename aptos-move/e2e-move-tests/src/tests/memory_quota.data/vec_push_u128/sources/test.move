module 0xbeef::test {
    use std::vector;

    public entry fun just_under_quota() {
        let v = vector::empty();
        let i = 0;

        while (i < 624999) {
            vector::push_back(&mut v, 0u128);
            i = i + 1;
        }
    }

    public entry fun just_above_quota() {
        let v = vector::empty();
        let i = 0;

        while (i < 625001) {
            vector::push_back(&mut v, 0u128);
            i = i + 1;
        }
    }
}
