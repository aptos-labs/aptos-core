//# publish
module 0x42::concrete_test {
    struct Pair<T, R> has drop, copy, store, key {
        first: T,
        second: R,
    }

    // Concrete receiver: only works for Pair<u64, bool>
    fun describe(self: &Pair<u64, bool>): u64 {
        if (self.second) { self.first } else { 0 }
    }

    fun flip(self: &mut Pair<u64, bool>) {
        self.second = !self.second;
    }

    fun into_first(self: Pair<u64, bool>): u64 {
        self.first
    }

    // Chain test helper — generic receiver on a different struct
    struct Wrapper<T> has drop, copy {
        inner: T,
    }

    fun unwrap<T: copy + drop>(self: Wrapper<T>): T {
        self.inner
    }

    // === Test functions ===

    fun test_basic() {
        let p = Pair<u64, bool> { first: 42, second: true };
        assert!(p.describe() == 42, 0);
    }

    fun test_mut_receiver() {
        let p = Pair<u64, bool> { first: 42, second: true };
        assert!(p.describe() == 42, 0);
        p.flip();
        assert!(p.describe() == 0, 1);
    }

    fun test_consume_receiver() {
        let p = Pair<u64, bool> { first: 99, second: true };
        assert!(p.into_first() == 99, 0);
    }

    fun test_chain() {
        let p = Pair<u64, bool> { first: 7, second: true };
        let w = Wrapper { inner: p };
        // Chain: unwrap (generic) then describe (concrete)
        assert!(w.unwrap().describe() == 7, 0);
    }

    fun test_vector_index() {
        let p1 = Pair<u64, bool> { first: 10, second: true };
        let p2 = Pair<u64, bool> { first: 20, second: false };
        let v = vector[p1, p2];
        assert!(v[0].describe() == 10, 0);
        assert!(v[1].describe() == 0, 1);  // second is false → returns 0
    }

    fun test_mut_through_vector() {
        let p = Pair<u64, bool> { first: 5, second: false };
        let v = vector[p];
        v[0].flip();
        assert!(v[0].describe() == 5, 0);
    }
}

//# run --verbose -- 0x42::concrete_test::test_basic

//# run --verbose -- 0x42::concrete_test::test_mut_receiver

//# run --verbose -- 0x42::concrete_test::test_consume_receiver

//# run --verbose -- 0x42::concrete_test::test_chain

//# run --verbose -- 0x42::concrete_test::test_vector_index

//# run --verbose -- 0x42::concrete_test::test_mut_through_vector
