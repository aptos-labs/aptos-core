// Human-facing baseline for optional missing-loop-invariant evidence.
// flag: --inference
// flag: --loop-invariant-evidence=3
// flag: -T=20
module 0x42::loop_invariant_evidence {
    fun count_down_together(x: u64, y: u64): (u64, u64) {
        while (x > 0 && y > 0) {
            x = x - 1;
            y = y - 1;
        };
        (x, y)
    }
}
