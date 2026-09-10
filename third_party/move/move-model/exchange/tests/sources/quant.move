module 0x42::quant {
    fun id(x: u64): u64 {
        x
    }
    spec id {
        ensures result == x;
        ensures forall y: u64: y + 1 > y;
        ensures exists y: u64: y == result;
    }
}
