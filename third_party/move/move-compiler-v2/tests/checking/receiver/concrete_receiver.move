module 0x42::m {
    struct G<T, R> has drop, copy { x: T, y: R }

    // === Positive: basic concrete receiver ===
    fun get_x(self: &G<u64, bool>): u64 { self.x }
    fun get_y(self: &G<u64, bool>): bool { self.y }
    fun consume(self: G<u64, bool>): u64 { self.x }
    fun set_x(self: &mut G<u64, bool>, v: u64) { self.x = v }

    // === Positive: concrete with nested types ===
    struct Inner has drop, copy { val: u64 }
    fun get_inner_val(self: &G<Inner, u64>): u64 { self.x.val }

    // === Positive: concrete with vector type arg ===
    fun sum_first(self: &G<vector<u64>, bool>): u64 { *&self.x[0] }

    // === Positive: call site tests ===
    fun test_basic_call() {
        let g = G<u64, bool> { x: 42, y: true };
        assert!(g.get_x() == 42, 0);
        assert!(g.get_y() == true, 1);
    }

    fun test_ref_call() {
        let g = G<u64, bool> { x: 42, y: true };
        let r = &g;
        assert!(r.get_x() == 42, 0);
    }

    fun test_mut_ref_call() {
        let g = G<u64, bool> { x: 42, y: true };
        g.set_x(99);
        assert!(g.get_x() == 99, 0);
    }

    fun test_consume_call() {
        let g = G<u64, bool> { x: 42, y: true };
        assert!(g.consume() == 42, 0);
    }

    fun test_nested_type_arg() {
        let g = G<Inner, u64> { x: Inner { val: 10 }, y: 20 };
        assert!(g.get_inner_val() == 10, 0);
    }
}
