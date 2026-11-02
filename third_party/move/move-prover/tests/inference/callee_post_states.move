// The post-state of a `&mut` argument after a call is named by an equation
// when the caller leaves it unchanged; otherwise it is bound existentially
// together with the callee's `ensures_of`.
// inference-reject-mutation: s.a = 1; => s.a = 2;
// inference-reject-mutation: set_a(s, 3); => set_a(s, 4);
// inference-reject-mutation: take_b(s) + 1 => take_b(s) + 2
// inference-reject-mutation: S { a: 1, b: 2 } => S { a: 1, b: 7 }
module 0x42::callee_post_states {
    struct S has drop {
        a: u64,
        b: u64,
    }

    fun inc_b(s: &mut S) {
        s.b = s.b + 1;
    }

    fun set_a(s: &mut S, x: u64) {
        s.a = x;
    }

    fun take_b(s: &mut S): u64 {
        let b = s.b;
        s.b = 0;
        b
    }

    fun call_then_write(s: &mut S) {
        inc_b(s);
        s.a = 1;
    }

    fun write_then_call(s: &mut S) {
        s.a = 5;
        inc_b(s);
    }

    fun two_calls(s: &mut S) {
        inc_b(s);
        set_a(s, 3);
    }

    fun call_then_read(s: &mut S): u64 {
        inc_b(s);
        s.b
    }

    fun result_and_effect(s: &mut S): u64 {
        take_b(s) + 1
    }

    fun local_call(): u64 {
        let x = S { a: 1, b: 2 };
        inc_b(&mut x);
        x.b
    }
}
