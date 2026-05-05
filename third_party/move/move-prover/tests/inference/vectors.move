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

    // borrow: aborts iff out of range; result is v[i]
    fun get<T>(v: &vector<T>, i: u64): &T {
        vector::borrow(v, i)
    }

    // borrow safe wrapper
    fun first<T>(v: &vector<T>): &T {
        vector::borrow(v, 0)
    }

    // length used in arithmetic
    fun len_plus_one<T>(v: &vector<T>): u64 {
        vector::length(v) + 1
    }

    // pop_back: aborts iff empty; mutates v
    fun pop<T>(v: &mut vector<T>): T {
        vector::pop_back(v)
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
}
