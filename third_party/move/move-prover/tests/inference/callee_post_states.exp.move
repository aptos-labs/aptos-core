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
    spec inc_b(s: &mut S) {
        pragma opaque = true;
        ensures [inferred] s == update_field(old(s), b, old(s).b + 1);
        aborts_if [inferred] s.b == MAX_U64;
    }


    fun set_a(s: &mut S, x: u64) {
        s.a = x;
    }
    spec set_a(s: &mut S, x: u64) {
        pragma opaque = true;
        ensures [inferred] s == update_field(old(s), a, x);
        aborts_if [inferred] false;
    }


    fun take_b(s: &mut S): u64 {
        let b = s.b;
        s.b = 0;
        b
    }
    spec take_b(s: &mut S): u64 {
        pragma opaque = true;
        ensures [inferred] result == old(s).b;
        ensures [inferred] s == update_field(old(s), b, 0);
        aborts_if [inferred] false;
    }


    fun call_then_write(s: &mut S) {
        inc_b(s);
        s.a = 1;
    }
    spec call_then_write(s: &mut S) {
        pragma opaque = true;
        ensures [inferred = sathard] exists x: S: ensures_of<inc_b>(old(s), x) && s == update_field(x, a, 1);
        aborts_if [inferred] aborts_of<inc_b>(s);
    }


    fun write_then_call(s: &mut S) {
        s.a = 5;
        inc_b(s);
    }
    spec write_then_call(s: &mut S) {
        pragma opaque = true;
        ensures [inferred] ensures_of<inc_b>(update_field(old(s), a, 5), s);
        aborts_if [inferred] aborts_of<inc_b>(update_field(s, a, 5));
    }


    fun two_calls(s: &mut S) {
        inc_b(s);
        set_a(s, 3);
    }
    spec two_calls(s: &mut S) {
        pragma opaque = true;
        ensures [inferred = sathard] exists x: S: (..S1 |~ ensures_of<inc_b>(old(s), x)) && (S1.. |~ ensures_of<set_a>(x, 3, s));
        aborts_if [inferred] aborts_of<inc_b>(s);
    }


    fun call_then_read(s: &mut S): u64 {
        inc_b(s);
        s.b
    }
    spec call_then_read(s: &mut S): u64 {
        pragma opaque = true;
        ensures [inferred] result == s.b;
        ensures [inferred] ensures_of<inc_b>(old(s), s);
        aborts_if [inferred] aborts_of<inc_b>(s);
    }


    fun result_and_effect(s: &mut S): u64 {
        take_b(s) + 1
    }
    spec result_and_effect(s: &mut S): u64 {
        pragma opaque = true;
        ensures [inferred] result == result_of<take_b>(old(s)) + 1;
        ensures [inferred] ensures_of<take_b>(old(s), result_of<take_b>(old(s)), s);
        aborts_if [inferred] result_of<take_b>(s) == MAX_U64;
    }


    fun local_call(): u64 {
        let x = S { a: 1, b: 2 };
        inc_b(&mut x);
        x.b
    }
    spec local_call(): u64 {
        pragma opaque = true;
        ensures [inferred = sathard] exists x: S: ensures_of<inc_b>(S{a: 1, b: 2}, x) && result == x.b;
        aborts_if [inferred] aborts_of<inc_b>(S{a: 1, b: 2});
    }

}
/*
Verification: Succeeded.
Mutation: Rejected by postcondition.
Mutation: Rejected by postcondition.
Mutation: Rejected by postcondition.
Mutation: Rejected by postcondition.
*/
