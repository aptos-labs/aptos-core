

module 0x100::M10 {
    struct Many<T1, T2, T3, T4, T5, T6> has copy, drop {
        a: T1,
        b: T2,
        c: T3,
        d: T4,
        e: T5,
        f: T6,
        x: bool,
    }

    public fun new(in1: u8, in2: u16, in3: u32, in4: u64, in5: u128, in6: u256): Many<u8, u16, u32, u64, u128, u256> {
        let t1 = Many<u8, u16, u32, u64, u128, u256> {
            a: in1, b: in2, c: in3, d: in4, e: in5, f: in6, x: true
        };
        t1
    }

    public fun get_a(s: Many<u8, u16, u32, u64, u128, u256>): u8 {
        s.a
    }
    public fun get_b(s: Many<u8, u16, u32, u64, u128, u256>): u16 {
        s.b
    }
    public fun get_c(s: Many<u8, u16, u32, u64, u128, u256>): u32 {
        s.c
    }
    public fun get_d(s: Many<u8, u16, u32, u64, u128, u256>): u64 {
        s.d
    }
    public fun get_e(s: Many<u8, u16, u32, u64, u128, u256>): u128 {
        s.e
    }
    public fun get_f(s: Many<u8, u16, u32, u64, u128, u256>): u256 {
        s.f
    }
    public fun get_x(s: Many<u8, u16, u32, u64, u128, u256>): bool {
        s.x
    }
}

script {
    use 0x100::M10;

    fun main() {
        let t1 = M10::new(1, 2, 3, 4, 5, 6);

        let ta = M10::get_a(t1);
        assert!(ta == 1, 0xf00);

        let tb = M10::get_b(t1);
        assert!(tb == 2, 0xf01);

        let tc = M10::get_c(t1);
        assert!(tc == 3, 0xf02);

        let td = M10::get_d(t1);
        assert!(td == 4, 0xf03);

        let te = M10::get_e(t1);
        assert!(te == 5, 0xf04);

        let tf = M10::get_f(t1);
        assert!(tf == 6, 0xf05);

        let tx = M10::get_x(t1);
        assert!(tx == true , 0xf07);
    }
}
