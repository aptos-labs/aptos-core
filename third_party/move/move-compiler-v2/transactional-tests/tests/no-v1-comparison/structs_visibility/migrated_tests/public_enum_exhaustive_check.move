//# publish
module 0x42::test {
    public struct S0 has drop {}

    public struct S1<A, B> has drop {
        x: A,
        y: B
    }

    public enum E has drop {
        V1{ x: u8, y: S1<u8, bool>},
        V2 {
            x: u8,
            y: S0
        }
    }

}

//# publish
module 0x42::test_bug_14296 {
    use 0x42::test::E;
    use 0x42::test::S1;

    fun extract_last_u8(y: &E): u8 {
        match (y) {
            E::V1{ x: _, y: S1 { x, y: _}} => *x,
            E::V2 { y: _, x: _ } => 1,
        }
    }
}
