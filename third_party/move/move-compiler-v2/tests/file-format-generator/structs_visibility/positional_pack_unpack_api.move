module 0x42::m1 {
    public struct Pair(u64, bool) has copy, drop;

    public struct Wrapper<T: drop>(T) has drop;

    public struct NestedPair<T: drop>(Pair, Wrapper<T>) has drop;

    public struct VecWrap<T: drop>(vector<T>) has drop;
}

module 0x42::m2 {
    use 0x42::m1::Pair;
    use 0x42::m1::Wrapper;
    use 0x42::m1::NestedPair;
    use 0x42::m1::VecWrap;

    public fun try_pack_unpack_pair() {
        let p = Pair(1, true);
        let Pair(x, y) = p;
        assert!(x == 1, 1);
        assert!(y == true, 2);
    }

    public fun try_pack_unpack_wrapper() {
        let w = Wrapper(100);
        let Wrapper(x) = w;
        assert!(x == 100, 3);
    }

    public fun try_nested_pair_unpack() {
        let p = Pair(5, false);
        let w = Wrapper(vector[42, 43]);
        let n = NestedPair(p, w);
        let NestedPair(Pair(a, b), Wrapper(v)) = n;
        assert!(a == 5, 4);
        assert!(b == false, 5);
        assert!(v[1] == 43, 6);
    }

    public fun try_vecwrap_unpack() {
        let vw = VecWrap(vector[10, 20, 30]);
        let VecWrap(v) = vw;
        assert!(v[0] == 10, 7);
        assert!(v[2] == 30, 8);
    }

    public fun try_return_positional(): Pair {
        Pair(77, false)
    }

    public fun try_unpack_from_return() {
        let Pair(x, y) = try_return_positional();
        assert!(x == 77, 9);
        assert!(y == false, 10);
    }

    public fun use_as_param(p: Pair) {
        let Pair(a, b) = p;
        assert!(a == 88, 11);
        assert!(b == true, 12);
    }

    public fun try_pass_as_arg() {
        let p = Pair(88, true);
        use_as_param(p);
    }
}
