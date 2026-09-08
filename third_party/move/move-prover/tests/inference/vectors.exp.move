// Spec inference for `std::vector` bytecode-instruction natives (and
// `singleton` / `is_empty` / `contains`). Exercises the direct WP path in
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


    // is_empty: never aborts; result is exactly the length test implemented
    // by the intrinsic Boogie procedure.
    fun isempty<T>(v: &vector<T>): bool {
        vector::is_empty(v)
    }
    spec isempty<T>(v: &vector<T>): bool {
        pragma opaque = true;
        ensures [inferred] result == (len(v) == 0);
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


    // The second guard establishes `i < n - 1`. Normalizing offsets on both
    // sides must prove both `i + 1 < n` and that `i + 1` cannot overflow, so
    // the vector read adds no vacuous abort condition.
    fun guarded_next(v: &vector<u64>, i: u64): u64 {
        let n = vector::length(v);
        if (n == 0) { return 0 };
        if (i >= n - 1) { return 0 };
        *vector::borrow(v, i + 1)
    }
    spec guarded_next(v: &vector<u64>, i: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == (if (len(v) == 0 || i >= len(v) - 1) 0 else v[i + 1]);
        aborts_if [inferred] false;
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


    // Results from consecutive mutating calls must each refer to that call's
    // own pre-state, while the final vector state chains through both calls.
    fun pop_two<T>(v: &mut vector<T>): (T, T) {
        let first = vector::pop_back(v);
        let second = vector::pop_back(v);
        (first, second)
    }
    spec pop_two<T>(v: &mut vector<T>): (T, T) {
        pragma opaque = true;
        ensures [inferred] result_1 == old(v)[len(old(v)) - 1];
        ensures [inferred] result_2 == old(v)[0..len(old(v)) - 1][len(old(v)[0..len(old(v)) - 1]) - 1];
        ensures [inferred] v == old(v)[0..len(old(v)) - 1][0..len(old(v)[0..len(old(v)) - 1]) - 1];
        aborts_if [inferred] len(v) == 0;
        aborts_if [inferred] len(v[0..len(v) - 1]) == 0;
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


    // Every pragma intrinsic in std::vector must be modeled internally. These
    // wrappers ensure inference never asks for source contracts on intrinsics.
    fun reverse<T>(v: &mut vector<T>) {
        vector::reverse(v)
    }
    spec reverse<T>(v: &mut vector<T>) {
        pragma opaque = true;
        ensures [inferred] v == reverse_vector(old(v));
        aborts_if [inferred] false;
    }


    fun reverse_slice<T>(v: &mut vector<T>, left: u64, right: u64) {
        vector::reverse_slice(v, left, right)
    }
    spec reverse_slice<T>(v: &mut vector<T>, left: u64, right: u64) {
        pragma opaque = true;
        ensures [inferred] v == (if (left == right) old(v) else concat(old(v)[0..left], concat(reverse_vector(old(v)[left..right]), old(v)[right..len(old(v))])));
        aborts_if [inferred] left > right || left != right && right > len(v);
    }


    fun append<T>(v: &mut vector<T>, other: vector<T>) {
        vector::append(v, other)
    }
    spec append<T>(v: &mut vector<T>, other: vector<T>) {
        pragma opaque = true;
        ensures [inferred] v == concat(old(v), other);
        aborts_if [inferred] false;
    }


    fun reverse_append<T>(v: &mut vector<T>, other: vector<T>) {
        vector::reverse_append(v, other)
    }
    spec reverse_append<T>(v: &mut vector<T>, other: vector<T>) {
        pragma opaque = true;
        ensures [inferred] v == concat(old(v), reverse_vector(other));
        aborts_if [inferred] false;
    }


    fun trim<T>(v: &mut vector<T>, new_len: u64): vector<T> {
        vector::trim(v, new_len)
    }
    spec trim<T>(v: &mut vector<T>, new_len: u64): vector<T> {
        pragma opaque = true;
        ensures [inferred] result == old(v)[new_len..len(old(v))];
        ensures [inferred] v == old(v)[0..new_len];
        aborts_if [inferred] new_len > len(v);
    }


    fun trim_reverse<T>(v: &mut vector<T>, new_len: u64): vector<T> {
        vector::trim_reverse(v, new_len)
    }
    spec trim_reverse<T>(v: &mut vector<T>, new_len: u64): vector<T> {
        pragma opaque = true;
        ensures [inferred] result == reverse_vector(old(v)[new_len..len(old(v))]);
        ensures [inferred] v == old(v)[0..new_len];
        aborts_if [inferred] new_len > len(v);
    }


    fun find_index<T>(v: &vector<T>, e: &T): (bool, u64) {
        vector::index_of(v, e)
    }
    spec find_index<T>(v: &vector<T>, e: &T): (bool, u64) {
        pragma opaque = true;
        ensures [inferred] result_1 == contains(v, e);
        ensures [inferred] result_2 == (if (contains(v, e)) index_of(v, e) else 0);
        aborts_if [inferred] false;
    }


    fun insert<T>(v: &mut vector<T>, i: u64, e: T) {
        vector::insert(v, i, e)
    }
    spec insert<T>(v: &mut vector<T>, i: u64, e: T) {
        pragma opaque = true;
        ensures [inferred] v == concat(concat(old(v)[0..i], vec(e)), old(v)[i..len(old(v))]);
        aborts_if [inferred] i > len(v);
    }


    fun remove<T>(v: &mut vector<T>, i: u64): T {
        vector::remove(v, i)
    }
    spec remove<T>(v: &mut vector<T>, i: u64): T {
        pragma opaque = true;
        ensures [inferred] result == old(v)[i];
        ensures [inferred] v == concat(old(v)[0..i], old(v)[i + 1..len(old(v))]);
        aborts_if [inferred] !in_range(v, i);
    }


    fun remove_value<T>(v: &mut vector<T>, e: &T): vector<T> {
        vector::remove_value(v, e)
    }
    spec remove_value<T>(v: &mut vector<T>, e: &T): vector<T> {
        pragma opaque = true;
        ensures [inferred] result == (if (contains(old(v), e)) vec(old(v)[index_of(old(v), e)]) else vec<T>());
        ensures [inferred] v == (if (contains(old(v), e)) concat(old(v)[0..index_of(old(v), e)], old(v)[index_of(old(v), e) + 1..len(old(v))]) else old(v));
        aborts_if [inferred] false;
    }


    fun swap_remove<T>(v: &mut vector<T>, i: u64): T {
        vector::swap_remove(v, i)
    }
    spec swap_remove<T>(v: &mut vector<T>, i: u64): T {
        pragma opaque = true;
        ensures [inferred] result == old(v)[i];
        ensures [inferred] v == update(old(v), i, old(v)[len(old(v)) - 1])[0..len(old(v)) - 1];
        aborts_if [inferred] !in_range(v, i);
    }


    fun rotate<T>(v: &mut vector<T>, rot: u64): u64 {
        vector::rotate(v, rot)
    }
    spec rotate<T>(v: &mut vector<T>, rot: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == len(old(v)) - rot;
        ensures [inferred] v == concat(old(v)[rot..len(old(v))], old(v)[0..rot]);
        aborts_if [inferred] rot > len(v);
    }


    fun rotate_slice<T>(v: &mut vector<T>, left: u64, rot: u64, right: u64): u64 {
        vector::rotate_slice(v, left, rot, right)
    }
    spec rotate_slice<T>(v: &mut vector<T>, left: u64, rot: u64, right: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == left + (right - rot);
        ensures [inferred] v == concat(old(v)[0..left], concat(concat(old(v)[rot..right], old(v)[left..rot]), old(v)[right..len(old(v))]));
        aborts_if [inferred] left > rot || rot > right || right > len(v);
    }


    fun move_range<T>(
        from: &mut vector<T>,
        removal_position: u64,
        count: u64,
        to: &mut vector<T>,
        insert_position: u64,
    ) {
        vector::move_range(from, removal_position, count, to, insert_position)
    }
    spec move_range<T>(from: &mut vector<T>, removal_position: u64, count: u64, to: &mut vector<T>, insert_position: u64) {
        pragma opaque = true;
        ensures [inferred] from == concat(old(from)[0..removal_position], old(from)[removal_position + count..len(old(from))]);
        ensures [inferred] to == concat(old(to)[0..insert_position], concat(old(from)[removal_position..removal_position + count], old(to)[insert_position..len(old(to))]));
        aborts_if [inferred] removal_position + count > len(from) || insert_position > len(to);
    }

}
/*
Verification: Succeeded.
*/
