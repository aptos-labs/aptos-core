//# publish
module 0xdecaf::eval {

    public fun test01(): u64 {
        let x = 1;
        x + {x = x + 1; x} + {x = x + 1; x}
    }

    public fun test02(): u64 {
        let x = 1;
        {x = x + 1; x} + x + {x = x + 1; x}
    }

    public fun test03(): u64 {
        let x = 1;
        {x + {x = x + 1; x}} + {x = x + 1; x}
    }

    public fun test04(): u64 {
        let x = 1;
        (x + {x = x + 1; x}) + {x = x + 1; x}
    }

    public fun test05(): u64 {
        let x = 1;
        let (a, b, c) = (x, {x = x + 1; x}, {x = x + 1; x});
        a + b + c
    }

    fun inc(x: &mut u64): u64 {
        *x = *x + 1;
        *x
    }

    public fun test06(): u64 {
        let x = 1;
        x + inc(&mut x) + inc(&mut x)
    }

    struct S has drop {
        x: u64,
        y: u64,
        z: u64,
    }

    public fun test07(): u64 {
        let x = 1;
        let S {x, y, z} = S { x, y: {x = x + 1; x}, z: {x = x + 1; x} };
        x + y + z
    }

    public fun test08(): u64 {
        let x = 1;
        let S {x, y, z} = S { x, y: inc(&mut x), z: inc(&mut x) };
        x + y + z
    }

    public fun test09(): u64 {
        let x = 1;
        let s = S { x, y: {x = x + 1; x}, z: {x = x + 1; x} };
        let S {x, y, z} = s;
        x + y + z
    }

    public fun test10(): u64 {
        let x = 1;
        let s = S { x, y: inc(&mut x), z: inc(&mut x) };
        let x;
        let y;
        let z;
        S {x, y, z} = s;
        x + y + z
    }

    public fun test11(): u64 {
        let a = 1;
        let x;
        let y;
        let z;
        (x, y, z) = (a, {a = a + 1; a}, {a = a + 1; a});
        x + y + z
    }

    public fun test12(): u64 {
        let x = 1;
        let S {y, x, z} = S { x, y: {x = x + 1; x}, z: {x = x + 1; x} };
        x + y + z
    }

    public fun test13(): u64 {
        let x = 1;
        x + {x = inc(&mut x) + 1; x} + {x = inc(&mut x) + 1; x}
    }

    public fun test14(): u64 {
        let x = 1;
        {x = inc(&mut x) + 1; x} + x + {x = inc(&mut x) + 1; x}
    }

    fun add2(x: u64, y: u64): u64 {
        x + y
    }

    fun add3(x: u64, y: u64, z: u64): u64 {
        x + y + z
    }

    public fun test15(): u64 {
        let x = 1;
        x + add3(x, {x = inc(&mut x); add3(x, {x = x + 1; x}, {x = x + 1; x})}, {x = inc(&mut x); add3({x = x + 1; x}, x, {x = x + 1; x})}) + {x = inc(&mut x) + 1; x}
    }

    fun inc_x(self: &mut S) {
        self.x = self.x + 1;
    }

    public fun test16(): u64 {
        let s = S { x: 1, y: 2, z: 3 };
        s.x + {inc_x(&mut s); s.x} + {inc_x(&mut s); s.x}
    }

    public fun test17(): u64 {
        let x = 1;
        add3(x, {x = add2(x, 1); x}, {x = add2(x, 1); x})
    }

    public fun test18(p: u64): vector<u64> {
        vector[p, {p = p + 1; p}, {p = p + 1; p}]
    }

    public fun test19(): bool {
        let x = 1;
        {x = x << 1; x} < {x = x << 1; x}
    }

    public fun test20(p: bool): bool {
        (!p && {p = p && false; p}) || {p = !p; !p}
    }
}

//# run 0xdecaf::eval::test01

//# run 0xdecaf::eval::test02

//# run 0xdecaf::eval::test03

//# run 0xdecaf::eval::test04

//# run 0xdecaf::eval::test05

//# run 0xdecaf::eval::test06

//# run 0xdecaf::eval::test07

//# run 0xdecaf::eval::test08

//# run 0xdecaf::eval::test09

//# run 0xdecaf::eval::test10

//# run 0xdecaf::eval::test11

//# run 0xdecaf::eval::test12

//# run 0xdecaf::eval::test13

//# run 0xdecaf::eval::test14

//# run 0xdecaf::eval::test15

//# run 0xdecaf::eval::test16

//# run 0xdecaf::eval::test17

//# run 0xdecaf::eval::test18 --args 1

//# run 0xdecaf::eval::test19

//# run 0xdecaf::eval::test20 --args true

//# run 0xdecaf::eval::test20 --args false
