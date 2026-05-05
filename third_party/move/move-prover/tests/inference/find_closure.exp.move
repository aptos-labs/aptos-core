// Test spec inference for a generic `find` over a vector with a closure
// predicate, using behavioral predicates (`result_of`, `aborts_of`) in the
// agent-supplied loop invariants.
// flag: -T=20
// flag: --aptos
module 0x42::find_closure {
    use std::vector;

    public fun find<T>(v: &vector<T>, pred: |&T|bool has copy + drop): u64 {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            if (pred(vector::borrow(v, i))) {
                return i
            };
            i = i + 1;
        } spec {
            invariant [inferred] i <= n;
            invariant [inferred] n == len(v);
            invariant [inferred] forall j: u64 where j < i: !result_of<pred>(v[j]);
            invariant [inferred] forall j: u64 where j < i: !aborts_of<pred>(v[j]);
        };
        n
    }
    spec find<T>(v: &vector<T>, pred: |&T|bool has copy + drop): u64 {
        pragma opaque = true;
        ensures [inferred] (forall x: u64: x < len(v) ==> !result_of<pred>(v[x])) && (forall x: u64: x < len(v) ==> !aborts_of<pred>(v[x])) ==> result == len(v);
        ensures [inferred = sathard] forall y: u64: (forall x: u64: x < y ==> !result_of<pred>(v[x])) && ((forall x: u64: x < y ==> !aborts_of<pred>(v[x])) && (y < len(v) && result_of<pred>(v[y]))) ==> result == y;
        aborts_if [inferred = sathard] exists y: u64: (forall x: u64: x < y ==> !result_of<pred>(v[x])) && ((forall x: u64: x < y ==> !aborts_of<pred>(v[x])) && (y < len(v) && aborts_of<pred>(v[y])));
        aborts_if [inferred = sathard] exists y: u64: (forall x: u64: x < y ==> !result_of<pred>(v[x])) && ((forall x: u64: x < y ==> !aborts_of<pred>(v[x])) && (y < len(v) && !in_range(v, y)));
    }

}
/*
Verification: Succeeded.
*/
