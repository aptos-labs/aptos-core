//# publish
module 0xc0ffee::m {
    fun and(x: bool, y: bool): bool {
        x && y
    }

    public fun test(): bool {
        let x = 1;
        and({x = x - 1; x == 0}, {x = x + 3; x == 3}) && {x = x * 2; x == 6}
    }
}

//# run 0xc0ffee::m::test
