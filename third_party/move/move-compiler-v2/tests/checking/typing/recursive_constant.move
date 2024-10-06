address 0x42 {
module M {
    const X: u64 = Y;
    const Y: u64 = X;
    const Z: u64 = 0;
    const F: u64 = F;
    const X1: u64 = {
       Z + A
    };
    const A: u64 = B + C;
    const B: u64 = X1;
    const C: u64 = Z + B;

    public fun get_x(): u64 {
        X
    }

    public fun get_y(): u64 {
        Y
    }
}
}
