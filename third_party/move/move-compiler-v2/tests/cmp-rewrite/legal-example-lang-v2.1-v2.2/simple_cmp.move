module 0xcffa::m {

    struct Test1 has drop {
        a: u8
    }

    struct Test2 has drop {
        a: vector<u8>
    }

     public fun eq1(x: u64, y: u64): u64 {
        if(x == y)
            x
        else
            y
    }

    public fun eq2(x: u64, y: u64): bool {
        &x == &y
    }

    public fun eq3<T: drop + copy>(x: T, y: T): bool {
        x == y
    }

    public fun eq4<T: drop + copy>(x: T, y: T): bool {
        &x == &y
    }

    public fun eq5(x: bool, y: bool): bool {
        x == y
    }

    public fun eq6(x: address, y: address): bool {
        x == y
    }

    public fun eq7(x: vector<u8>, y: vector<u8>): bool {
        x == y
    }

    public fun eq8(x: vector<vector<u8>>, y: vector<vector<u8>>): bool {
        x == y
    }

    public fun eq9(x: Test1, y: Test1): bool {
        x == y
    }

    public fun eq10(x: Test2, y: Test2): bool {
        x == y
    }

    public fun neq1(x: u64, y: u64): bool {
        x != y
    }

    public fun neq2(x: u64, y: u64): bool {
        &x != &y
    }

    public fun neq3<T: drop + copy>(x: T, y: T): bool {
        x != y
    }

    public fun neq4<T: drop + copy>(x: T, y: T): bool {
        &x != &y
    }

    public fun neq5(x: bool, y: bool): bool {
        x != y
    }

    public fun neq6(x: address, y: address): bool {
        x != y
    }

    public fun neq7(x: vector<u8>, y: vector<u8>): bool {
        x != y
    }

    public fun neq8(x: vector<vector<u8>>, y: vector<vector<u8>>): bool {
        x != y
    }

    public fun neq9(x: Test1, y: Test1): bool {
        x != y
    }

    public fun neq10(x: Test2, y: Test2): bool {
        x != y
    }
}
