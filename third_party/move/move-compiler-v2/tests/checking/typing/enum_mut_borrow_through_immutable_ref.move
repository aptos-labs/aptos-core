module 0x42::m {
    enum Data has drop {
        V { x: u64 }
    }

    fun test() {
        let d = Data::V { x: 1 };
        let imm = &d;
        let r = &mut imm.x;
        *r = 2;
    }
}
