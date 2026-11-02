// Match guards cannot modify pattern bindings or the matched value.
// Each function containing a match must fail guard checking.
// These cases cover assignment, mutable borrows, writes through references,
// moves, and closure captures.
module 0xc0ffee::m {
    enum W has copy, drop { W1(u64), W2(u64) }
    struct Coin has drop { v: u64 }
    enum E has drop { V(Coin), N }
    struct S has copy, drop { f: u64 }

    fun clamp(amount: &mut u64, limit: u64) { if (*amount > limit) *amount = limit; }
    fun bump(v: &mut u64): bool { *v = *v + 1; false }
    fun weird(w: &mut W): bool { *w = W::W2(10); false }
    fun consume(c: Coin): bool { c.v > 3 }
    fun set_f(f: &mut u64) { *f = 9; }

    // Pattern-variable assignments across primitive, tuple, and enum matches.

    fun assign_primitive(requested: u64, limit: u64): u64 {
        match (requested) {
            amount if ({ if (amount > limit) amount = limit; amount <= limit }) => amount,
            _ => 0,
        }
    }

    fun assign_primitive_tuple(a: u64, b: u64): u64 {
        match ((a, b)) {
            (x, y) if ({ x = y; x > 1 }) => x,
            _ => 0,
        }
    }

    fun assign_mixed_tuple(w: &W, k: u64): u64 {
        match ((w, k)) {
            (W::W1(_), y) if ({ y = 1; true }) => y,
            _ => 0,
        }
    }

    fun assign_enum_value(w: W): u64 {
        match (w) {
            W::W1(x) if ({ x = 5; true }) => x,
            _ => 0,
        }
    }

    fun assign_enum_ref(w: &W): u64 {
        match (w) {
            W::W1(x) if ({ x = &7; true }) => *x,
            _ => 0,
        }
    }

    // Mutable borrows, reference writes, reference transfers, and moves of bindings.

    fun borrow_primitive(requested: u64, limit: u64): u64 {
        match (requested) {
            amount if ({ clamp(&mut amount, limit); true }) => amount,
            _ => 0,
        }
    }

    fun write_through_primitive(x: &mut u64): u64 {
        match (x) {
            r if ({ *r = 2; false }) => 0,
            2 => 20,
            _ => 99,
        }
    }

    fun write_through_enum(w: &mut W): u64 {
        match (w) {
            W::W1(p) if ({ *p = 7; false }) => 0,
            W::W1(q) if (bump(q)) => 1,
            W::W1(s) if ({ let t = s; *t = 1; false }) => 2,
            _ => 99,
        }
    }

    fun move_payload(e: E): u64 {
        match (e) {
            E::V(c) if (consume(c)) => 1,
            _ => 0,
        }
    }

    // Changes to the matched value through discriminator variables.

    fun rebind_matched(w: &mut W, other: &mut W): u64 {
        match (w) {
            W::W1(_) if ({ w = other; false }) => 0,
            _ => 99,
        }
    }

    fun mutate_matched_value(x: u64): u64 {
        let y = x;
        match (y) {
            1 if ({ y = 2; false }) => 0,
            1 if ({ clamp(&mut y, 0); false }) => 1,
            _ => 99,
        }
    }

    fun mutate_through_matched_ref(w: &mut W): u64 {
        match (w) {
            W::W1(_) if (weird(w)) => 0,
            W::W1(_) if ({ *w = W::W2(1); false }) => 1,
            W::W1(_) if ({ w.0 = 3; false }) => 2,
            W::W1(_) if ({ let d = w; false }) => 3,
            _ => 99,
        }
    }

    fun nested_match_on_matched_ref(w: &mut W): u64 {
        match (w) {
            W::W1(_) if (match (w) { W::W1(p) => { *p = 1; true }, _ => false }) => 0,
            _ => 99,
        }
    }

    fun mutate_matched_tuple_elements(w: &mut W, k: u64): u64 {
        match ((w, k)) {
            (W::W1(_), 1) if (weird(w)) => 0,
            (W::W1(_), _) if ({ k = 2; false }) => 1,
            _ => 99,
        }
    }

    // Mutable reborrows of discriminator references and pattern bindings.

    fun reborrow_matched_primitive(x: &mut u64): u64 {
        match (x) {
            1 if (bump(&mut *x)) => 0,
            2 => 20,
            _ => 99,
        }
    }

    fun reborrow_binding(x: &mut u64): u64 {
        match (x) {
            r if (bump(&mut *r)) => 0,
            2 => 20,
            _ => 99,
        }
    }

    fun reborrow_matched_enum(w: &mut W): u64 {
        match (w) {
            W::W1(_) if (weird(&mut *w)) => 0,
            W::W1(p) if (bump(&mut *p)) => 1,
            W::W2(_) => 20,
            _ => 99,
        }
    }

    // Field writes and mutable field borrows of by-value variables.

    fun field_write_matched_value(s: S, k: u64): u64 {
        match ((s, k)) {
            (S { f: _ }, 1) if ({ s.f = 9; false }) => 0,
            (S { f }, _) => f,
        }
    }

    fun field_write_binding(s: S): u64 {
        match (s) {
            t if ({ t.f = 5; true }) => t.f,
            _ => 0,
        }
    }

    fun field_borrow_binding(s: S): u64 {
        match (s) {
            t if ({ set_f(&mut t.f); true }) => t.f,
            _ => 0,
        }
    }

    fun field_write_payload(e: E): u64 {
        match (e) {
            E::V(c) if ({ c.v = 5; true }) => c.v,
            _ => 0,
        }
    }

    // Closure captures that would move non-copyable pattern bindings.

    fun capture_payload_in_closure(e: E): u64 {
        match (e) {
            E::V(c) if ({ let f = || c.v > 3; f() }) => 1,
            _ => 0,
        }
    }

    fun capture_payload_in_typed_closure(e: E): u64 {
        match (e) {
            E::V(c) if ({ let f: ||bool has copy + drop = || c.v > 3; f() }) => 1,
            _ => 0,
        }
    }

    fun consume_e(e: E): bool { match (e) { E::V(c) => c.v > 3, E::N => false } }

    fun capture_matched_parameter_in_closure(e: E): u64 {
        match (e) {
            E::V(_) if ({ let f = || consume_e(e); f() }) => 1,
            _ => 0,
        }
    }
}
