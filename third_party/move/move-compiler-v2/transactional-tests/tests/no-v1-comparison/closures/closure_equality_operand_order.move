//# publish
module 0xc0ffee::m {
    struct DropStore has drop, store { x: u64 }

    public fun with_capture(d: DropStore, y: u64): u64 {
        let DropStore { x } = d;
        x + y
    }

    // A lambda takes the other operand's type in either operand order.
    fun lambda_first(d: DropStore, g: |u64|u64 has drop): bool {
        (|y| with_capture(d, y)) == g
    }

    fun lambda_second(d: DropStore, g: |u64|u64 has drop): bool {
        g == (|y| with_capture(d, y))
    }

    fun lambda_first_neq(d: DropStore, g: |u64|u64 has drop): bool {
        (|y| with_capture(d, y)) != g
    }

    fun lambda_second_neq(d: DropStore, g: |u64|u64 has drop): bool {
        g != (|y| with_capture(d, y))
    }

    // Closure equality is structural, so the compared lambdas must capture a local: a lambda over a
    // struct literal may be lifted into a capture-free function instead.
    fun main() {
        let same = DropStore { x: 7 };
        assert!(lambda_first(DropStore { x: 7 }, |y| with_capture(same, y)), 0);
        let same = DropStore { x: 7 };
        assert!(lambda_second(DropStore { x: 7 }, |y| with_capture(same, y)), 1);
        let other = DropStore { x: 8 };
        assert!(!lambda_first(DropStore { x: 7 }, |y| with_capture(other, y)), 2);
        let other = DropStore { x: 8 };
        assert!(!lambda_second(DropStore { x: 7 }, |y| with_capture(other, y)), 3);
        let same = DropStore { x: 7 };
        assert!(!lambda_first_neq(DropStore { x: 7 }, |y| with_capture(same, y)), 4);
        let other = DropStore { x: 8 };
        assert!(lambda_second_neq(DropStore { x: 7 }, |y| with_capture(other, y)), 5);
    }
}

//# run 0xc0ffee::m::main
