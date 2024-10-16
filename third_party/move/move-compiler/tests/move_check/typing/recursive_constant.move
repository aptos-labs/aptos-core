address 0x42 {
module M {
    const X: u64 = Y;
    const Y: u64 = X;
    const Z: u64 = 0;

    public fun get_x(): u64 {
        X
    }

    public fun get_y(): u64 {
        Y
    }
}
}
