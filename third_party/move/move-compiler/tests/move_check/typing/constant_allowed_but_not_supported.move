address 0x42 {
module M {
    const X: u64 = 0;
    const Y: u64 = 0;
    const Z: u64 = 0;
    const C: u64 = {
        move X;
        copy Y;
        Z;
        0
    };
}
}
