// Writes to two fields through one mutable reference to a global resource:
// the postcondition must state both, not only the last one.
// inference-reject-mutation: c.a = x; => c.a = y;
// inference-reject-mutation: c.a = a; => c.a = b;
module 0x42::global_field_writes {
    struct Config has key {
        a: u64,
        b: u64,
    }

    enum Versioned has key {
        V1 { a: u64, b: u64 },
    }

    fun set_both(x: u64, y: u64) {
        let c = &mut Config[@0x42];
        c.a = x;
        c.b = y;
    }

    fun set_both_variant(p: u64, q: u64) {
        let c = &mut Versioned[@0x42];
        c.a = p;
        c.b = q;
    }

    fun checked(x: u64): u64 {
        assert!(x > 0, 1);
        x
    }
    spec checked {
        pragma opaque;
        ensures result >= x;
        aborts_if x == 0;
    }

    // The written values come from calls without a functional contract, which
    // label the states between them, so the update starts from a labeled
    // state. Each written field still has to be stated.
    fun set_or_publish_checked(s: &signer, u: u64, v: u64) {
        let a = checked(u);
        let b = checked(v);
        if (!exists<Versioned>(@0x42)) {
            move_to(s, Versioned::V1 { a, b });
        } else {
            let c = &mut Versioned[@0x42];
            c.a = a;
            c.b = b;
        }
    }

    fun set_or_publish_variant(s: &signer, u: u64, v: u64) {
        if (!exists<Versioned>(@0x42)) {
            move_to(s, Versioned::V1 { a: u, b: v });
        } else {
            let c = &mut Versioned[@0x42];
            c.a = u;
            c.b = v;
        }
    }
}
