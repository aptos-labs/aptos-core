//# publish
module 0xc0ffee::m {
    public fun test1a(): vector<u64> {
        let f = || std::vector::empty();
        f()
    }

    public fun test1b(): vector<u64> {
        let f = std::vector::empty;
        f()
    }

    public fun test2a(): vector<u64> {
        let f = || std::vector::empty<u64>();
        f()
    }

    public fun test2b(): vector<u64> {
        let f = std::vector::empty<u64>;
        f()
    }

    public fun test3a<T>(): vector<T> {
        let f = || std::vector::empty();
        f()
    }

    public fun test3b<T>(): vector<T> {
        let f = std::vector::empty;
        f()
    }

    public fun test4a<T>(): vector<T> {
        let f = || std::vector::empty<T>();
        f()
    }

    public fun test4b<T>(): vector<T> {
        let f = std::vector::empty<T>;
        f()
    }

    fun apply<T>(f: || vector<T>): vector<T> {
        f()
    }

    public fun test5(): vector<u64> {
        let f = || apply(std::vector::empty);
        f()
    }

    public fun test6(): vector<u64> {
        let f = std::vector::singleton;
        f(42)
    }

    struct Resource(u64);

    public fun test7() {
        let f = std::vector::destroy_empty<Resource>;
        f(vector[]);
    }

    public fun test8(): u64 {
        let f = std::vector::length;
        f(&vector[1, 2, 3])
    }

    public fun test9a(): vector<u64> {
        let v = vector[1, 2, 3];
        let f = |v, i, j| std::vector::swap(v, i, j);
        f(&mut v, 0, 2);
        v
    }

    public fun test9b(): vector<u64> {
        let v = vector[1, 2, 3];
        let f = std::vector::swap;
        f(&mut v, 0, 2);
        v
    }

    public fun test10(): vector<u64> {
        let e = 4;
        let v = vector[1, 2, 3];
        let f = |v| std::vector::push_back(v, e);
        f(&mut v);
        v
    }
}

//# run 0xc0ffee::m::test1a

//# run 0xc0ffee::m::test1b

//# run 0xc0ffee::m::test2a

//# run 0xc0ffee::m::test2b

//# run 0xc0ffee::m::test3a --type-args u64

//# run 0xc0ffee::m::test3b --type-args u16

//# run 0xc0ffee::m::test4a --type-args u32

//# run 0xc0ffee::m::test4b --type-args u128

//# run 0xc0ffee::m::test5

//# run 0xc0ffee::m::test6

//# run 0xc0ffee::m::test7

//# run 0xc0ffee::m::test8

//# run 0xc0ffee::m::test9a

//# run 0xc0ffee::m::test9b

//# run 0xc0ffee::m::test10
