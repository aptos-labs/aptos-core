module 0x42::m {
    fun f1(): u8 {
        1 + 2 + 3 // Should automatically infer that it's all u8
    }

    fun f2(): bool {
        // Below constant expression should default to u64
        1 % 2 == 0
    }

    fun f3(x: u8, y: u32) : u32 {
        // Should infer r to be a u32
        let r = 1 << x;
        y + r
    }

    fun f4(x: u8) : u8 {
        let r = x + 1 + 2;
        let error = 257 + r; // Should error
        error
    }
}
