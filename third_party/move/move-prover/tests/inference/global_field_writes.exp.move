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
    spec set_both(x: u64, y: u64) {
        pragma opaque = true;
        modifies Config[@0x42];
        ensures [inferred] update<Config>(@0x42, update_field(update_field(old(Config[@0x42]), a, x), b, y));
        aborts_if [inferred] !exists<Config>(@0x42);
    }


    fun set_both_variant(p: u64, q: u64) {
        let c = &mut Versioned[@0x42];
        c.a = p;
        c.b = q;
    }
    spec set_both_variant(p: u64, q: u64) {
        pragma opaque = true;
        modifies Versioned[@0x42];
        ensures [inferred] update<Versioned>(@0x42, update_field(update_field(old(Versioned[@0x42]), a, p), b, q));
        aborts_if [inferred] !exists<Versioned>(@0x42);
    }


    fun checked(x: u64): u64 {
        assert!(x > 0, 1);
        x
    }
    spec checked {
        pragma opaque;
        ensures result >= x;
        aborts_if x == 0;
        ensures [inferred] x > 0 ==> result == x;
        aborts_if [inferred] x == 0;
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
    spec set_or_publish_checked(s: &signer, u: u64, v: u64) {
        use 0x1::signer;
        pragma opaque = true;
        modifies Versioned[signer::address_of(s)];
        modifies Versioned[@0x42];
        ensures [inferred] (S2 |~ !exists<Versioned>(@0x42)) ==> {
            let a = signer::address_of(s);
            let b = Versioned::V1{a: ..S1 |~ result_of<checked>(u), b: S1..S2 |~ result_of<checked>(v)};
            S2.. |~ publish<Versioned>(a, b)
        };
        ensures [inferred] (S2 |~ exists<Versioned>(@0x42)) ==> Versioned[@0x42].a == (..S1 |~ result_of<checked>(u)) && Versioned[@0x42].b == (S1..S2 |~ result_of<checked>(v));
        aborts_if [inferred] S1 |~ (aborts_of<checked>(v));
        aborts_if [inferred] aborts_of<checked>(u);
        aborts_if [inferred] (S2 |~ !exists<Versioned>(@0x42)) && (S2 |~ exists<Versioned>(signer::address_of(s)));
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
    spec set_or_publish_variant(s: &signer, u: u64, v: u64) {
        use 0x1::signer;
        pragma opaque = true;
        modifies Versioned[signer::address_of(s)];
        modifies Versioned[@0x42];
        ensures [inferred] !old(exists<Versioned>(@0x42)) ==> publish<Versioned>(signer::address_of(s), Versioned::V1{a: u, b: v});
        ensures [inferred] old(exists<Versioned>(@0x42)) ==> update<Versioned>(@0x42, update_field(update_field(old(Versioned[@0x42]), a, u), b, v));
        aborts_if [inferred] !exists<Versioned>(@0x42) && exists<Versioned>(signer::address_of(s));
    }

}
/*
Verification: Succeeded.
Mutation: Rejected by postcondition.
Mutation: Rejected by postcondition.
*/
