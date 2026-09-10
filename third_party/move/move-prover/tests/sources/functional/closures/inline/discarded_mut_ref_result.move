module 0x42::discarded_mut_ref_result {
    use std::vector;

    struct S has copy, drop {
        value: u64,
    }

    fun increment(s: &mut S): &mut S {
        s.value = s.value + 1;
        s
    }
    spec increment {
        aborts_if s.value == MAX_U64;
        ensures s.value == old(s).value + 1;
    }

    inline fun each_mut(v: &mut vector<S>, f: |&mut S|) {
        let i = 0;
        while (i < vector::length(v)) {
            f(vector::borrow_mut(v, i));
            i = i + 1;
        } spec {
            invariant i <= len(v);
            invariant len(v) == len(old(v));
            invariant forall j in 0..i: ensures_of<f>(old(v)[j], v[j]);
            invariant forall j in 0..i: !aborts_of<f>(old(v)[j]);
            invariant forall j in i..len(v): v[j] == old(v)[j];
        };
    }

    fun run(v: &mut vector<S>) {
        each_mut(v, |s| {
            increment(s);
        });
    }
    spec run {
        pragma aborts_if_is_partial;
    }
}
