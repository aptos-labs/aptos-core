// Behavioural predicates are not allowed over `std::vector` functions that
// have direct spec-language equivalents (`empty`, `length`, `borrow`,
// `borrow_mut`, `push_back`, `pop_back`, `destroy_empty`, `swap`, plus
// `singleton` and `contains`). Each BP form below is rejected with a
// diagnostic pointing the user at the spec-language alternative
// (`len(v)` instead of `result_of<vector::length>(v)`,
// `!in_range(v, i)` instead of `aborts_of<vector::borrow>(v, i)`, …).
module 0x42::bp_bytecode_natives {
    use std::vector;

    // ====================================================================
    // empty<T>()
    // ====================================================================

    fun empty_aborts<T>(): vector<T> {
        vector::empty()
    }
    spec empty_aborts {
        aborts_if aborts_of<vector::empty<T>>();
    }

    // ====================================================================
    // length<T>(v)
    // ====================================================================

    fun length_no_abort<T>(v: &vector<T>): u64 {
        vector::length(v)
    }
    spec length_no_abort {
        aborts_if aborts_of<vector::length<T>>(v);
    }

    fun length_result<T>(v: &vector<T>): u64 {
        vector::length(v)
    }
    spec length_result {
        aborts_if false;
        ensures result == result_of<vector::length<T>>(v);
    }

    // ====================================================================
    // borrow<T>(v, i)
    // ====================================================================

    fun borrow_aborts<T>(v: &vector<T>, i: u64): &T {
        vector::borrow(v, i)
    }
    spec borrow_aborts {
        aborts_if aborts_of<vector::borrow<T>>(v, i);
    }

    fun borrow_safe<T>(v: &vector<T>, i: u64): &T {
        vector::borrow(v, i)
    }
    spec borrow_safe {
        requires !aborts_of<vector::borrow<T>>(v, i);
        aborts_if false;
    }

    // ====================================================================
    // borrow_mut<T>(v, i)
    // ====================================================================

    fun borrow_mut_aborts<T>(v: &mut vector<T>, i: u64): &mut T {
        vector::borrow_mut(v, i)
    }
    spec borrow_mut_aborts {
        aborts_if aborts_of<vector::borrow_mut<T>>(v, i);
    }

    // ====================================================================
    // push_back<T>(v, e)
    // ====================================================================

    fun push_back_no_abort<T: drop>(v: &mut vector<T>, e: T) {
        vector::push_back(v, e)
    }
    spec push_back_no_abort {
        aborts_if aborts_of<vector::push_back<T>>(v, e);
    }

    /// `ensures_of` over push_back captures the post-state via the
    /// in/out-param convention: `ensures_of<push_back>(v, e, v_post)`
    /// rewrites to `v_post == concat(v, vec(e))`.
    fun push_back_ensures<T: drop>(v: &mut vector<T>, e: T) {
        vector::push_back(v, e)
    }
    spec push_back_ensures {
        aborts_if false;
        ensures ensures_of<vector::push_back<T>>(old(v), e, v);
    }

    // ====================================================================
    // pop_back<T>(v)
    // ====================================================================

    fun pop_back_aborts<T>(v: &mut vector<T>): T {
        vector::pop_back(v)
    }
    spec pop_back_aborts {
        aborts_if aborts_of<vector::pop_back<T>>(v);
    }

    fun pop_back_safe<T>(v: &mut vector<T>): T {
        vector::pop_back(v)
    }
    spec pop_back_safe {
        requires !aborts_of<vector::pop_back<T>>(v);
        aborts_if false;
    }

    // ====================================================================
    // destroy_empty<T>(v)
    // ====================================================================

    fun destroy_empty_aborts<T>(v: vector<T>) {
        vector::destroy_empty(v)
    }
    spec destroy_empty_aborts {
        aborts_if aborts_of<vector::destroy_empty<T>>(v);
    }

    // ====================================================================
    // swap<T>(v, i, j)
    // ====================================================================

    fun swap_aborts<T>(v: &mut vector<T>, i: u64, j: u64) {
        vector::swap(v, i, j)
    }
    spec swap_aborts {
        aborts_if aborts_of<vector::swap<T>>(v, i, j);
    }

    // ====================================================================
    // singleton<T>(e) — non-bytecode-instruction native, but spec-equivalent
    // ====================================================================

    fun mk_one<T: drop>(e: T): vector<T> {
        vector::singleton(e)
    }
    spec mk_one {
        aborts_if aborts_of<vector::singleton<T>>(e);
        ensures result == result_of<vector::singleton<T>>(e);
    }

    // ====================================================================
    // contains<T>(v, &e) — non-bytecode-instruction native, but spec-equivalent
    // ====================================================================

    fun is_member<T>(v: &vector<T>, e: &T): bool {
        vector::contains(v, e)
    }
    spec is_member {
        aborts_if aborts_of<vector::contains<T>>(v, e);
        ensures result == result_of<vector::contains<T>>(v, e);
    }
}
