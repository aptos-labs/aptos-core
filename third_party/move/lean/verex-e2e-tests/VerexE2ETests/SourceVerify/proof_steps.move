/// Proof steps of the proof language: `calc`, `forall`, `if`, `witness`,
/// splits, `let` of a variant, the `congr` and `contradiction` closers, and
/// the names a step reads: a structure's fields, the results of repeated
/// calls, a generic callee's precondition, the value a `&mut` parameter's
/// calls write back and its value at the entry; a guarded `apply`. Each obligation is quantified, so
/// that it remains after preparation and its block runs.
module 0x42::proof_steps {
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
        again
    }
    spec twice {
        requires x + 2 <= MAX_U64;
        aborts_if false;
        ensures forall i: u64 where i <= x: i + 2 <= result;
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

    fun pick3(i: u64): u64 {
        if (i == 0) 10 else if (i == 1) 20 else 30
    }
    spec pick3 {
        requires i < 3;
        aborts_if false;
        ensures result == 10 + 10 * i;
    }

    enum Shape has drop, copy {
        Circle { r: u64 },
        Square { side: u64 },
        Dot,
    }

    spec fun shape(k: u64): Shape {
        if (k == 0) Shape::Dot else Shape::Circle { r: k }
    }

    spec fun size(s: Shape): u64 {
        if (s is Circle) s.r else if (s is Square) s.side else 0
    }

    fun make(k: u64): Shape {
        if (k == 0) Shape::Dot else Shape::Circle { r: k }
    }
    spec make {
        aborts_if false;
        ensures result == shape(k);
    }

    fun made_size(k: u64): u64 {
        match (make(k)) {
            Shape::Circle { r } => r,
            _ => 0,
        }
    }
    spec made_size {
        aborts_if false;
        ensures forall i: u64 where i == 0: result + i == size(shape(k));
    }

    fun radius(k: u64): u64 {
        match (make(k + 1)) {
            Shape::Circle { r } => r,
            _ => 0,
        }
    }
    spec radius {
        requires k < 100;
        aborts_if false;
        ensures forall i: u64 where i == 0: result + i == k + 1;
    }

    spec fun h(x: num): num;

    fun scaled(a: u64, b: u64): u64 {
        b
    }
    spec scaled {
        requires a == 2;
        requires b < 100;
        aborts_if false;
        ensures forall i: u64 where i == 0: h(a * b) + i == h(2 * b);
    }

    fun positive(x: u64): u64 {
        x + 1
    }
    spec positive {
        requires x < 10;
        aborts_if false;
        ensures forall i: u64 where i == 0: result > i;
    }

    struct Counter has drop {
        n: u64,
    }

    fun increment(c: &mut Counter) {
        c.n = c.n + 1;
    }
    spec increment {
        requires c.n < 100;
        aborts_if false;
        ensures c.n == old(c.n) + 1;
    }

    fun increment_twice(c: &mut Counter) {
        increment(c);
        increment(c);
    }
    spec increment_twice {
        requires c.n < 50;
        aborts_if false;
        ensures forall i: u64 where i == 0: c.n + i == old(c.n) + 2;
    }

    spec module {
        lemma double_bound(x: u64) {
            requires x < 10;
            ensures x * 2 < 20;
        }
    }

    fun double_small(x: u64): u64 {
        if (x < 10) { x * 2 } else { 0 }
    }
    spec double_small {
        requires x < 100;
        aborts_if false;
        ensures forall i: u64 where i == 0: result + i < 20;
    }
}
