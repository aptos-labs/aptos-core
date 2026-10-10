// Tests that an uninterpreted spec function respects Move value equality
// over vector-containing argument types: under a non-extensional vector
// theory two representations can be Move-equal (`$IsEqual`) without being
// raw-(SMT-)equal — e.g. a havocked loop-state element constrained by an
// invariant against its snapshot — and an uninterpreted function is only
// congruent over raw equality on its own. The backend emits a congruence
// axiom for uninterpreted spec functions used by behavioral predicates when
// parameter types lack native equality; without it, `ensures_of` fails below.
module 0x42::uninterpreted_spec_fun_congruence {
    use std::vector;

    struct GhostInput has copy, drop { runtime: u64 }
    spec GhostInput {
        ghost proof: u64;
    }

    spec module {
        fun spec_scramble(data: vector<u8>): vector<u8>;
        fun observe_ghost(input: GhostInput): u64;
        axiom forall input: GhostInput: observe_ghost(input) == input.proof;
    }

    struct S has copy, drop { data: vector<u8> }

    native fun scramble_internal(data: vector<u8>): vector<u8>;
    spec scramble_internal {
        pragma opaque;
        aborts_if [abstract] false;
        ensures result == spec_scramble(data);
    }

    fun scramble(s: &mut S) {
        s.data = scramble_internal(s.data);
    }
    spec scramble {
        aborts_if false;
        ensures s.data == spec_scramble(old(s).data);
    }

    inline fun for_each_mut<T>(v: &mut vector<T>, f: |&mut T|) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(vector::borrow_mut(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant len(v) == len(old(v));
            invariant n == len(v);
            invariant forall j in 0..i: ensures_of<f>(old(v)[j], v[j]);
            invariant forall j in 0..i: !aborts_of<f>(old(v)[j]);
            invariant forall j in i..n: v[j] == old(v)[j];
        };
    }

    /// The element `f` is applied to is loop state, related to the
    /// invariant's `old(v)[j]` only by Move equality; proving the lambda's
    /// attached spec instance needs the congruence of `spec_scramble`.
    fun scramble_all(v: &mut vector<S>) {
        for_each_mut(v, |e| {
            scramble(e);
        } spec {
            aborts_if false;
            ensures e.data == spec_scramble(old(e).data);
        });
    }
    spec scramble_all {
        aborts_if false;
        ensures forall j in 0..len(v): v[j].data == spec_scramble(old(v)[j].data);
    }

    /// Canary: congruence must not prove too much.
    fun scramble_all_wrong(v: &mut vector<S>) {
        for_each_mut(v, |e| {
            scramble(e);
        } spec {
            aborts_if false;
            ensures e.data == spec_scramble(old(e).data);
        });
    }
    spec scramble_all_wrong {
        aborts_if false;
        ensures forall j in 0..len(v): v[j].data == old(v)[j].data; // error: elements are scrambled
    }

    /// Move equality ignores ghost fields, while the uninterpreted function
    /// is explicitly axiomatized to observe one. A `$IsEqual` congruence
    /// axiom for `observe_ghost` would make these assumptions inconsistent
    /// and prove the false assertion.
    fun ghost_sensitive_function_is_not_move_congruent(a: GhostInput, b: GhostInput) {
        spec {
            assume a == b;
            assume a.proof != b.proof;
            assert false; // error: differing ghost fields remain consistent
        };
    }
}
