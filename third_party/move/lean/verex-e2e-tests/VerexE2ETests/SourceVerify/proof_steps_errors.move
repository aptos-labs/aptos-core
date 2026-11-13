/// Proof steps that do not hold, each reported at its step: a `forall`'s
/// assertion, a `calc` link, an `if` branch's assertion, witnesses that do
/// not establish the existential, assertions over a structure's fields and
/// repeated calls' results, and an `assume`, which a proof file does not
/// support yet.
module 0x42::proof_steps_errors {
    fun bump(x: u64): u64 {
        x + 1
    }
    spec bump {
        aborts_if x + 1 > MAX_U64;
        ensures forall i: u64 where i <= x: i < result;
        ensures forall i: u64 where i <= x: i + 1 <= result;
    }

    fun clamp(x: u64): u64 {
        if (x > 10) { 10 } else { x }
    }
    spec clamp {
        ensures forall i: u64 where i > 10: result < i;
    }

    fun pick(x: u64): u64 {
        x + 1
    }
    spec pick {
        aborts_if x + 1 > MAX_U64;
        ensures exists y: u64: y < result;
        ensures forall i: u64 where i <= x: exists j: u64: i < j && j <= result;
        ensures forall i: u64 where i <= x: i < result;
    }

    struct Pair has copy, drop { a: u64, b: u64 }

    fun total(p: Pair): u64 {
        p.a + p.b
    }
    spec total {
        aborts_if p.a + p.b > MAX_U64;
        ensures forall i: u64 where i <= p.a: i <= result;
    }

    fun inc(x: u64): u64 {
        x + 1
    }
    spec inc {
        pragma opaque;
        aborts_if x + 1 > MAX_U64;
        ensures result > x;
        ensures result <= x + 1;
    }

    fun twice(x: u64): u64 {
        let once = inc(x);
        let again = inc(once);
        again - 1
    }
    spec twice {
        requires x + 2 <= MAX_U64;
        aborts_if false;
        ensures forall i: u64 where i <= x: i + 1 <= result;
    }

    fun same<T: drop>(_x: T, n: u64): u64 {
        n
    }
    spec same {
        pragma opaque;
        requires exists y: T, k: u64: y == _x && 2 * k == n;
        aborts_if false;
        ensures result == n;
    }

    fun same_flag(m: u64): u64 {
        same(true, m + m + 2)
    }
    spec same_flag {
        requires m + m + 2 <= MAX_U64;
        aborts_if false;
        ensures result == m + m + 2;
    }
}
