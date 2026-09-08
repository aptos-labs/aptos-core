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

    // is_empty: never aborts; result is exactly the length test implemented
    // by the intrinsic Boogie procedure.
    fun isempty<T>(v: &vector<T>): bool {
        vector::is_empty(v)
    }

    // borrow: aborts iff out of range; result is v[i]
    fun get<T>(v: &vector<T>, i: u64): &T {
        vector::borrow(v, i)
    }

    // borrow safe wrapper
    fun first<T>(v: &vector<T>): &T {
        vector::borrow(v, 0)
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

    // length used in arithmetic
    fun len_plus_one<T>(v: &vector<T>): u64 {
        vector::length(v) + 1
    }

    // pop_back: aborts iff empty; mutates v
    fun pop<T>(v: &mut vector<T>): T {
        vector::pop_back(v)
    }

    // Results from consecutive mutating calls must each refer to that call's
    // own pre-state, while the final vector state chains through both calls.
    fun pop_two<T>(v: &mut vector<T>): (T, T) {
        let first = vector::pop_back(v);
        let second = vector::pop_back(v);
        (first, second)
    }

    // push_back: never aborts; mutates v
    fun push<T: drop>(v: &mut vector<T>, e: T) {
        vector::push_back(v, e)
    }

    // swap: aborts iff either index out of range; mutates v
    fun do_swap<T>(v: &mut vector<T>, i: u64, j: u64) {
        vector::swap(v, i, j)
    }

    // singleton: never aborts; result is vec(e)
    fun wrap<T: drop>(e: T): vector<T> {
        vector::singleton(e)
    }

    // contains: never aborts; result is contains(v, e)
    fun has<T>(v: &vector<T>, e: &T): bool {
        vector::contains(v, e)
    }

    // Every pragma intrinsic in std::vector must be modeled internally. These
    // wrappers ensure inference never asks for source contracts on intrinsics.
    fun reverse<T>(v: &mut vector<T>) {
        vector::reverse(v)
    }

    fun reverse_slice<T>(v: &mut vector<T>, left: u64, right: u64) {
        vector::reverse_slice(v, left, right)
    }

    fun append<T>(v: &mut vector<T>, other: vector<T>) {
        vector::append(v, other)
    }

    fun reverse_append<T>(v: &mut vector<T>, other: vector<T>) {
        vector::reverse_append(v, other)
    }

    fun trim<T>(v: &mut vector<T>, new_len: u64): vector<T> {
        vector::trim(v, new_len)
    }

    fun trim_reverse<T>(v: &mut vector<T>, new_len: u64): vector<T> {
        vector::trim_reverse(v, new_len)
    }

    fun find_index<T>(v: &vector<T>, e: &T): (bool, u64) {
        vector::index_of(v, e)
    }

    fun insert<T>(v: &mut vector<T>, i: u64, e: T) {
        vector::insert(v, i, e)
    }

    fun remove<T>(v: &mut vector<T>, i: u64): T {
        vector::remove(v, i)
    }

    fun remove_value<T>(v: &mut vector<T>, e: &T): vector<T> {
        vector::remove_value(v, e)
    }

    fun swap_remove<T>(v: &mut vector<T>, i: u64): T {
        vector::swap_remove(v, i)
    }

    fun rotate<T>(v: &mut vector<T>, rot: u64): u64 {
        vector::rotate(v, rot)
    }

    fun rotate_slice<T>(v: &mut vector<T>, left: u64, rot: u64, right: u64): u64 {
        vector::rotate_slice(v, left, rot, right)
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
}
