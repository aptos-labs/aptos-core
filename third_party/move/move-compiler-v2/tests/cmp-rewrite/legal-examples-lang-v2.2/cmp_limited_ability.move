module 0x00::test {

    struct Test has drop {
        a: u8,
        b: u16
    }

    public fun eq1(x: Test, y: Test): Test{
        if (x==y)
            x
        else
            y

    }

    public fun eq2(x: Test, y: Test): Test{
        if (&x==&y)
            x
        else
            y

    }
}

module 0x01::test {

    struct Test has drop {
        a: u8,
        b: u16
    }

    public fun eq1(x: vector<Test>, y: vector<Test>): vector<Test>{
        if (x==y)
            x
        else
            y

    }

    public fun eq2(x: vector<Test>, y: vector<Test>): vector<Test>{
        if (&x==&y)
            x
        else
            y

    }
}
