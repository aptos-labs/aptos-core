module 0x815::m {

    enum EnumSelf has copy, drop {
        A { self: u64}
    }

    enum EnumUseSelf has copy, drop {
       Self { self: EnumSelf }
    }

    spec EnumUseSelf {
        invariant self.self.self > 10;
    }

    fun test_enum_self() {
        let s = EnumSelf::A {
            self: 30
        };
        let self_s = EnumUseSelf::Self {
            self: s
        };
        let EnumUseSelf::Self {self: A {self} } = &mut self_s;
        *self = 10; // aborts
    }


}
