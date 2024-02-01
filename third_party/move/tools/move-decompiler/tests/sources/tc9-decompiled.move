module 0x12::tc9 {
    public fun foo() : u64 {
        let v0 = 0;
        let v1;
        loop {
            v1 = v0 + 1;
            v0 = v1;
            if (v1 / 2 == 0) {
                continue
            };
            if (v1 == 5) {
                break
            };
            v0 = v1 + 69 + v1;
        };
        v1 + 99
    }
    
    // decompiled from Move bytecode v6
}
