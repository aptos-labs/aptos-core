module 0xcfff::m {
    public fun eq1(): bool {
        1 == 1
    }

    public fun eq2(): bool {
        &1 == &1
    }

    public fun eq3<T: copy + drop>(_x: T, _y: T): bool {
        let x = vector<u8>[1, 2, 3, 4, 5, 6];
        let y = vector<u8>[1, 2, 3, 4, 5, 6];
        x == y
    }

    public fun eq4<T: copy + drop>(_x: T, _y: T): bool {
        let x = &vector<u8>[1, 2, 3, 4, 5, 6];
        let y = &vector<u8>[1, 2, 3, 4, 5, 6];
        *x == *y
    }

    struct Work(|u64|u64) has drop;

    inline fun eq5():bool {
        let work = Work(|x| x + 1);
        work == (|x| x + 1)
    }


}
