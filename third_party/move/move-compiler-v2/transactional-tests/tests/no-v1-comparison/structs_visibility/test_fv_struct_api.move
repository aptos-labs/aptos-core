//# publish
module 0x42::m1 {
    public struct Predicate<T>(|&T| bool) has copy, drop;

    public struct Holder<T>(Predicate<T>) has copy, drop;

}

//# publish
module 0x42::m2 {
    use 0x42::m1::Predicate;
    use 0x42::m1::Holder;

    fun apply_pred(p: Predicate<u64>, val: u64): bool {
        p(&val)
    }

    fun apply_holder(h: Holder<u64>, val: u64): bool {
        let Holder(p) = h;
        p(&val)
    }

    fun try_direct_assignment_and_call() {
        let p: Predicate<u64> = |x| *x < 100;
        assert!(p(&99), 1);
        assert!(!p(&101), 2);
    }

    fun try_pass_to_apply() {
        let p: Predicate<u64> = |x| *x % 2 == 0;
        assert!(apply_pred(p, 4), 5);
        assert!(!apply_pred(p, 5), 6);
    }

    fun try_nested_wrapper() {
        let h = Holder(|x| *x == 42);
        assert!(apply_holder(h, 42), 7);
        assert!(!apply_holder(h, 99), 8);
    }

    fun try_unpack_wrapper() {
        let p: Predicate<u64> = |x| *x != 0;
        let Predicate(f) = p;
        assert!(f(&1), 9);
        assert!(!f(&0), 10);
    }
}

//# run 0x42::m2::try_direct_assignment_and_call

//# run 0x42::m2::try_pass_to_apply

//# run 0x42::m2::try_nested_wrapper

//# run 0x42::m2::try_unpack_wrapper
