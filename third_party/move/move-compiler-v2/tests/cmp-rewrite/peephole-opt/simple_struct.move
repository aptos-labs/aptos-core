module 0xcffa::m {

    struct Test1 has copy, drop {
        a: u8
    }

    struct Test2 has copy, drop {
        a: vector<u8>
    }

    public fun eq1(x: vector<u8>, y: vector<u8>): vector<u8> {
        if (x == y)
            x
        else
            y
    }

    public fun eq2(x: vector<u8>, y: vector<u8>): bool{
        let eq = x == y;

        let neq = x != y;

        eq || neq
    }

    public fun eq3(x: vector<vector<u8>>, y: vector<vector<u8>>): vector<u8>{
        let a = x[0];
        let b = y[0];

        if (a == y[0] && b == x[0])
            a
        else
            b

    }

     public fun eq4(x: vector<u8>, y: vector<u8>): bool{
        let eq = x == y;

        let x1 = x;

        eq
    }

    public fun eq5(x: Test1, y: Test1): Test1 {
        if (x == y)
            x
        else
            y
    }

    public fun eq6(x: Test2, y: Test2): Test2 {
        if (x == y)
            x
        else
            y
    }
}
