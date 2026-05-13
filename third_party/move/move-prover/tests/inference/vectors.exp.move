// Spec inference for `std::vector` bytecode-instruction natives (and
// `singleton` / `contains`). Exercises the direct WP path in
// `spec_inference::try_wp_vector_intrinsic_call`; the expected
// `.exp.move` should contain only direct spec expressions
// (`!in_range(v, i)`, `len(v)`, `concat(v, vec(e))`, `update(...)`,
// `v[a..b]`, …) — never `aborts_of<…vector::…>`, `result_of<…vector::…>`,
// `ensures_of<…vector::…>`, or `requires_of<…vector::…>` — and must end
// with `Verification: Succeeded.`.
//
// flag: -T=20
// flag: --aptos
module 0x42::vectors {
    use std::vector;

    // length: never aborts; result is len(v)
    fun lengthof<T>(v: &vector<T>): u64 {
        vector::length(v)
    }
    spec lengthof<T>(v: &vector<T>): u64 {
        pragma opaque = true;
        ensures [inferred] result == len(v);
        aborts_if [inferred] false;
    }


    // borrow: aborts iff out of range; result is v[i]
    fun get<T>(v: &vector<T>, i: u64): &T {
        vector::borrow(v, i)
    }
    spec get<T>(v: &vector<T>, i: u64): &T {
        pragma opaque = true;
        ensures [inferred] result == v[i];
        aborts_if [inferred] !in_range(v, i);
    }


    // borrow safe wrapper
    fun first<T>(v: &vector<T>): &T {
        vector::borrow(v, 0)
    }
    spec first<T>(v: &vector<T>): &T {
        pragma opaque = true;
        ensures [inferred] result == v[0];
        aborts_if [inferred] !in_range(v, 0);
    }


    // length used in arithmetic
    fun len_plus_one<T>(v: &vector<T>): u64 {
        vector::length(v) + 1
    }
    spec len_plus_one<T>(v: &vector<T>): u64 {
        pragma opaque = true;
        ensures [inferred] result == len(v) + 1;
        aborts_if [inferred] len(v) == MAX_U64;
    }


    // pop_back: aborts iff empty; mutates v
    fun pop<T>(v: &mut vector<T>): T {
        vector::pop_back(v)
    }
    spec pop<T>(v: &mut vector<T>): T {
        pragma opaque = true;
        ensures [inferred] result == old(v)[len(old(v)) - 1];
        ensures [inferred] v == old(v)[0..len(old(v)) - 1];
        aborts_if [inferred] len(v) == 0;
    }


    // push_back: never aborts; mutates v
    fun push<T: drop>(v: &mut vector<T>, e: T) {
        vector::push_back(v, e)
    }
    spec push<T: drop>(v: &mut vector<T>, e: T) {
        pragma opaque = true;
        ensures [inferred] v == concat(old(v), vec(e));
        aborts_if [inferred] false;
    }


    // swap: aborts iff either index out of range; mutates v
    fun do_swap<T>(v: &mut vector<T>, i: u64, j: u64) {
        vector::swap(v, i, j)
    }
    spec do_swap<T>(v: &mut vector<T>, i: u64, j: u64) {
        pragma opaque = true;
        ensures [inferred] v == update(update(old(v), i, old(v)[j]), j, old(v)[i]);
        aborts_if [inferred] !in_range(v, i) || !in_range(v, j);
    }


    // singleton: never aborts; result is vec(e)
    fun wrap<T: drop>(e: T): vector<T> {
        vector::singleton(e)
    }
    spec wrap<T: drop>(e: T): vector<T> {
        pragma opaque = true;
        ensures [inferred] result == vec(e);
        aborts_if [inferred] false;
    }


    // contains: never aborts; result is contains(v, e)
    fun has<T>(v: &vector<T>, e: &T): bool {
        vector::contains(v, e)
    }
    spec has<T>(v: &vector<T>, e: &T): bool {
        pragma opaque = true;
        ensures [inferred] result == contains(v, e);
        aborts_if [inferred] false;
    }

}
/*
Verification: Succeeded.
*/
