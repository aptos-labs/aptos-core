module 0x42::test {
    struct S(bool, u8, address);

    struct T {
        x: bool,
        y: u8,
        z: address
    }

    fun extra_dotdot(x: S, y: T) {
        let S(_x, _, _, ..) = x;
        let S(.., _, _, _) = x;
        let S(_, .., _, _) = x;
        let T { x: _, y: _, z: _, .. } = y;
    }
}
