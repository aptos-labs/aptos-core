// Test that `0x1::vector` and `std::vector` resolve identically.
module 0xcafe::test {
    public entry fun some() {
        let v = vector[];
        0x1::vector::push_back(&mut v, 1);
        std::vector::push_back(&mut v, 2);
        assert!(v == vector[1, 2], 1);
    }
}
