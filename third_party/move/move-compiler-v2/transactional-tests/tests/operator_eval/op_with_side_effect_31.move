//# publish
module 0xc0ffee::m {
    struct S has drop {
        x: u64,
        y: u64,
        z: u64,
    }

    fun inc_x(self: &mut S) {
        self.x = self.x + 1;
    }

    public fun test1(): u64 {
        let s = S { x: 1, y: 2, z: 3 };
        s.x + {inc_x(&mut s); s.x} + {inc_x(&mut s); s.x}
    }

    fun inc_xx(self: &mut S, by: u64) {
        self.x = self.x + by;
    }

    public fun test2(): u64 {
        let s = S { x: 1, y: 2, z: 3 };
        s.x + {inc_xx(&mut s, 3); s.x} + {inc_xx(&mut s, 11); s.x}
    }
}

//# run 0xc0ffee::m::test1

//# run 0xc0ffee::m::test2
