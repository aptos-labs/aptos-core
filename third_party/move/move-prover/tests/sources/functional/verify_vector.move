// This file is created to verify the vector module in the standard library.
// This file is basically a clone of `stdlib/modules/vector.move` with renaming the module and function names.
// In this file, the functions with prefix of `verify_model` are verifying the corresponding built-in Boogie
// procedures that they inline (e.g., `verify_model_remove`).
// This file also verifies the actual Move implementations of non-native functions (e.g., `verify_remove`).
module 0x42::VerifyVector {
    use std::vector;

    fun verify_model_empty<Element>() : vector<Element> {
        vector::empty<Element>() // inlining the built-in Boogie procedure
    }
    spec verify_model_empty {
        ensures len(result) == 0;
    }

    // Return the length of the vector.
    fun verify_model_length<Element>(v: &vector<Element>): u64 {
        vector::length(v) // inlining the built-in Boogie procedure
    }
    spec verify_model_length {
        ensures result == len(v);
    }

    // Acquire an immutable reference to the ith element of the vector.
    fun verify_model_borrow<Element>(v: &vector<Element>, i: u64): &Element {
        vector::borrow(v, i) // inlining the built-in Boogie procedure
    }
    spec verify_model_borrow {
        aborts_if i >= len(v);
        ensures result == v[i]; // TODO: enough?
    }

    // Add an element to the end of the vector.
    fun verify_model_push_back<Element>(v: &mut vector<Element>, e: Element) {
        vector::push_back(v, e); // inlining the built-in Boogie procedure
    }
    spec verify_model_push_back {
        ensures len(v) == len(old(v)) + 1;
        ensures v[len(v)-1] == e;
        ensures old(v) == v[0..len(v)-1];
    }

    // Get mutable reference to the ith element in the vector, abort if out of bound.
    fun verify_model_borrow_mut<Element>(v: &mut vector<Element>, i: u64): &mut Element {
        vector::borrow_mut(v, i) // inlining the built-in Boogie procedure
    }
    spec verify_model_borrow_mut {
        aborts_if i >= len(v);
        ensures result == v[i]; // TODO: enough?
    }

    // Pop an element from the end of vector, abort if the vector is empty.
    fun verify_model_pop_back<Element>(v: &mut vector<Element>): Element {
        vector::pop_back(v) // inlining the built-in Boogie procedure
    }
    spec verify_model_pop_back {
        aborts_if len(v) == 0;
        ensures len(v) == len(old(v)) - 1;
        ensures result == old(v[len(v)-1]);
        ensures v == old(v[0..(len(v)-1)]);
    }

    // Destroy the vector, abort if not empty.
    fun verify_model_destroy_empty<Element>(v: vector<Element>) {
        vector::destroy_empty(v); // inlining the built-in Boogie procedure
    }
    spec verify_model_destroy_empty {
        aborts_if len(v) > 0;
        // TODO: anything else?
    }

    // Swaps the elements at the i'th and j'th indices in the vector.
    fun verify_model_swap<Element>(v: &mut vector<Element>, i: u64, j: u64) {
        vector::swap(v, i, j); // inlining the built-in Boogie procedure
    }
    spec verify_model_swap {
        aborts_if i >= len(v);
        aborts_if j >= len(v);
        ensures v == old(update(update(v,i,v[j]),j,v[i]));
    }

    // Return an vector of size one containing `e`
    fun verify_singleton<Element>(e: Element): vector<Element> {
        let v = vector::empty();
        vector::push_back(&mut v, e);
        v
    }
    spec verify_singleton {
        aborts_if false;
        ensures len(result) == 1;
        ensures result == vector[e];
    }

    // Reverses the order of the elements in the vector in place.
    fun verify_reverse<Element>(v: &mut vector<Element>) {
        let vlen = vector::length(v);
        if (vlen == 0) return ();

        let front_index = 0;
        let back_index = vlen -1;
        while ({
            spec {
                invariant front_index + back_index == vlen - 1;
                invariant forall i in 0..front_index: v[i] == old(v)[vlen-1-i];
                invariant forall i in 0..front_index: v[vlen-1-i] == old(v)[i];
                invariant forall j in front_index..back_index+1: v[j] == old(v)[j];
                invariant len(v) == vlen;
            };
            (front_index < back_index)
        }) {
            vector::swap(v, front_index, back_index);
            front_index = front_index + 1;
            back_index = back_index - 1;
        };
    }
    spec verify_reverse {
        aborts_if false;
        ensures forall i in 0..len(v): v[i] == old(v)[len(v)-1-i];
    }

    fun verify_reverse_with_unroll<Element>(v: &mut vector<Element>) {
        let vlen = vector::length(v);
        if (vlen == 0) return ();

        let front_index = 0;
        let back_index = vlen -1;
        while (front_index < back_index) {
            vector::swap(v, front_index, back_index);
            front_index = front_index + 1;
            back_index = back_index - 1;
        };
    }
    spec verify_reverse_with_unroll {
        pragma unroll=3;
        aborts_if false;
        ensures forall i in 0..len(v): v[i] == old(v)[len(v)-1-i];
    }

    // Reverses the order of the elements in the vector in place.
    fun verify_model_reverse<Element>(v: &mut vector<Element>) {
        vector::reverse(v); // inlining the built-in Boogie procedure
    }
    spec verify_model_reverse {
        aborts_if false;
        ensures forall i in 0..len(v): old(v[i]) == v[len(v)-1-i];
    }

    // Moves all of the elements of the `other` vector into the `v` vector.
    fun verify_append<Element>(v: &mut vector<Element>, other: &mut vector<Element>) {
        vector::reverse(other);
        while ({
            spec {
                invariant len(v) >= len(old(v));
                invariant len(other) <= len(old(other));
                invariant len(v) + len(other) == len(old(v)) + len(old(other));
                invariant forall k in 0..len(old(v)): v[k] == old(v)[k];
                invariant forall k in 0..len(other): other[k] == old(other)[len(old(other))-1-k];
                invariant forall k in len(old(v))..len(v): v[k] == old(other)[k-len(old(v))];
            };
            !vector::is_empty(other)
        }) {
            vector::push_back(v, vector::pop_back(other))
        };
    }
    spec verify_append {
        ensures len(v) == old(len(v)) + old(len(other));
        ensures v[0..len(old(v))] == old(v);
        ensures v[len(old(v))..len(v)] == old(other);
    }

    fun verify_append_with_unroll<Element>(v: &mut vector<Element>, other: vector<Element>) {
        let o = &mut other;
        vector::reverse(o);
        while (!vector::is_empty(o)) {
            vector::push_back(v, vector::pop_back(o))
        };
        vector::destroy_empty(other);
    }
    spec verify_append_with_unroll {
        pragma unroll=3;
        ensures len(v) == old(len(v)) + len(other);
        ensures v[0..len(old(v))] == old(v);
        ensures v[len(old(v))..len(v)] == other;
    }

    // Moves all of the elements of the `other` vector into the `lhs` vector.
    fun verify_model_append<Element>(lhs: &mut vector<Element>, other: vector<Element>) {
        vector::append(lhs, other) // inlining the built-in Boogie procedure
    }
    spec verify_model_append {
        ensures len(lhs) == old(len(lhs) + len(other));
        ensures lhs[0..len(old(lhs))] == old(lhs);
        ensures lhs[len(old(lhs))..len(lhs)] == other;
    }

    // Return true if the vector has no elements
    fun verify_is_empty<Element>(v: &vector<Element>): bool {
        vector::length(v) == 0
    }
    spec verify_is_empty {
        ensures result == (len(v) == 0);
    }

    // Return true if the vector has no elements
    fun verify_model_is_empty<Element>(v: &vector<Element>): bool {
        vector::is_empty(v) // inlining the built-in Boogie procedure
    }
    spec verify_model_is_empty {
        ensures result == (len(v) == 0);
    }

    // Return (true, i) if `e` is in the vector `v` at index `i`.
    // Otherwise returns (false, 0).
    fun verify_index_of<Element>(v: &vector<Element>, e: &Element): (bool, u64) {
        let i = 0;
        let len = vector::length(v);
        while ({
            spec {
                invariant !(exists j in 0..i: v[j]==e);
            };
            i < len
        }) {
            if (vector::borrow(v, i) == e) return (true, i);
            i = i + 1;
        };
        (false, 0)
    }
    spec verify_index_of {
        aborts_if false;
        ensures result_1 == (exists x in v: x==e); // whether v contains e or not
        ensures result_1 ==> v[result_2] == e; // if true, return the index where v contains e
        ensures result_1 ==> (forall i in 0..result_2: v[i]!=e); // ensure the smallest index
        ensures !result_1 ==> result_2 == 0; // return 0 if v does not contain e
    }

    fun verify_index_of_with_unroll<Element>(v: &vector<Element>, e: &Element): (bool, u64) {
        let i = 0;
        let len = vector::length(v);
        while (i < len) {
            if (vector::borrow(v, i) == e) return (true, i);
            i = i + 1;
        };
        (false, 0)
    }
    spec verify_index_of_with_unroll {
        pragma unroll=3;
        aborts_if false;
        ensures result_1 == (exists x in v: x==e); // whether v contains e or not
        ensures result_1 ==> v[result_2] == e; // if true, return the index where v contains e
        ensures result_1 ==> (forall i in 0..result_2: v[i]!=e); // ensure the smallest index
        ensures !result_1 ==> result_2 == 0; // return 0 if v does not contain e
    }

    fun verify_model_index_of<Element>(v: &vector<Element>, e: &Element): (bool, u64) {
        vector::index_of(v, e) // inlining the built-in Boogie procedure
    }
    spec verify_model_index_of {
        aborts_if false;
        ensures result_1 == (exists x in v: x==e); // whether v contains e or not
        ensures result_1 ==> v[result_2] == e; // if true, return the index where v contains e
        ensures result_1 ==> (forall i in 0..result_2: v[i]!=e); // ensure the smallest index
        ensures !result_1 ==> result_2 == 0; // return 0 if v does not contain e
    }

    // Return true if `e` is in the vector `v`
    fun verify_contains<Element>(v: &vector<Element>, e: &Element): bool {
        let i = 0;
        let len = vector::length(v);
        while ({
            spec {
               invariant !(exists j in 0..i: v[j]==e);
            };
            i < len
        }) {
            if (vector::borrow(v, i) == e) return true;
            i = i + 1;
        };
        spec {
           assert !(exists x in v: x==e);
        };
        false
    }
    spec verify_contains {
        aborts_if false;
        ensures result == (exists x in v: x==e);
    }

    fun verify_contains_with_unroll<Element>(v: &vector<Element>, e: &Element): bool {
        let i = 0;
        let len = vector::length(v);
        while (i < len) {
            if (vector::borrow(v, i) == e) return true;
            i = i + 1;
        };
        spec {
           assert !(exists x in v: x==e);
        };
        false
    }
    spec verify_contains_with_unroll {
        pragma unroll=3;
        aborts_if false;
        ensures result == (exists x in v: x==e);
    }

    // Return true if `e` is in the vector `v`
    fun verify_model_contains<Element>(v: &vector<Element>, e: &Element): bool {
        vector::contains(v, e) // inlining the built-in Boogie procedure.
    }
    spec verify_model_contains {
        aborts_if false;
        ensures result == (exists x in v: x==e);
    }

    // Remove the `i`th element E of the vector, shifting all subsequent elements
    // It is O(n) and preserves ordering
    fun verify_remove<Element>(v: &mut vector<Element>, j: u64): Element {
        let vlen = vector::length(v);
        let i = j;
        // i out of bounds; abort
        if (i >= vlen) abort 10;

        vlen = vlen - 1;
        while ({
            spec {
                invariant j <= i && i <= vlen;
                invariant vlen + 1 == len(v);
                invariant v[0..j] == old(v)[0..j];
                invariant forall k in j..i: v[k] == old(v)[k+1];
                invariant forall k in i+1..len(v): v[k] == old(v)[k];
                invariant v[i] == old(v)[j];
            };
            i < vlen
            }) {
            vector::swap(v, i, i + 1);
            i = i + 1;
        };
        vector::pop_back(v)
    }
    spec verify_remove {
        aborts_if j >= len(v);
        ensures len(v) == len(old(v)) - 1;
        ensures v[0..j] == old(v[0..j]);
        ensures v[j..len(v)] == old(v[j+1..len(v)]);
        ensures old(v[j]) == result;
    }

    fun verify_remove_with_unroll<Element>(v: &mut vector<Element>, j: u64): Element {
        let vlen = vector::length(v);
        let i = j;
        // i out of bounds; abort
        if (i >= vlen) abort 10;

        vlen = vlen - 1;
        while (i < vlen) {
            vector::swap(v, i, i + 1);
            i = i + 1;
        };
        vector::pop_back(v)
    }
    spec verify_remove_with_unroll {
        pragma unroll=3;
        aborts_if j >= len(v);
        ensures len(v) == len(old(v)) - 1;
        ensures v[0..j] == old(v[0..j]);
        ensures v[j..len(v)] == old(v[j+1..len(v)]);
        ensures old(v[j]) == result;
    }

    // Remove the `i`th element E of the vector, shifting all subsequent elements
    // It is O(n) and preserves ordering
    fun verify_model_remove<Element>(v: &mut vector<Element>, i: u64): Element {
        vector::remove(v, i) // inlining the built-in Boogie procedure.
    }
    spec verify_model_remove {
        aborts_if i >= len(v);
        ensures len(v) == len(old(v)) - 1;
        ensures v[0..i] == old(v[0..i]);
        ensures v[i..len(v)] == old(v[i+1..len(v)]);
        ensures old(v[i]) == result;
    }

    // Remove the `i`th element E of the vector by swapping it with the last element,
    // and then popping it off
    // It is O(1), but does not preserve ordering
    fun verify_swap_remove<Element>(v: &mut vector<Element>, i: u64): Element {
        let last_idx = vector::length(v) - 1;
        vector::swap(v, i, last_idx);
        vector::pop_back(v)
    }
    spec verify_swap_remove {
        aborts_if i >= len(v);
        ensures len(v) == len(old(v)) - 1;
        ensures v == old(update(v,i,v[len(v)-1])[0..len(v)-1]);
        ensures old(v[i]) == result;
    }

    // Remove the `i`th element E of the vector by swapping it with the last element,
    // and then popping it off
    // It is O(1), but does not preserve ordering
    fun verify_model_swap_remove<Element>(v: &mut vector<Element>, i: u64): Element {
        vector::swap_remove(v, i) // inlining the built-in Boogie procedure.
    }
    spec verify_model_swap_remove {
        aborts_if i >= len(v);
        ensures len(v) == len(old(v)) - 1;
        ensures v == old(update(v,i,v[len(v)-1])[0..len(v)-1]);
        ensures old(v[i]) == result;
    }

    fun vector_operator_in_function<Element: copy + drop>(e1: Element, e2: Element, e3: Element): vector<Element> {
        vector[e1, e2, e3]
    }

    fun spec_with_vector_operator<Element: copy + drop>(e1: Element, e2: Element, e3: Element): (vector<Element>, vector<Element>) {
        let v0 = vector::empty();
        let v3 = vector::empty();
        vector::push_back(&mut v3, e1);
        spec {
            assert v3 == vector[e1];
        };
        vector::push_back(&mut v3, e2);
        spec {
            assert v3 == vector[e1, e2];
        };
        vector::push_back(&mut v3, e3);
        spec {
            assert v3 == vector_operator_in_function(e1, e2, e3);
        };
        vector::push_back(&mut v3, e1);
        spec {
            assert v3 == vector[e1, e2, e3, e1];
        };
        vector::push_back(&mut v3, e2);
        spec {
            assert v3 == vector[e1, e2, e3, e1, e2];
        };
        (v3, v0)
    }
    spec spec_with_vector_operator {
        ensures result_2 == vector[];
        ensures result_1 == concat(vector_operator_in_function(e1, e2, e3), vector[e1, e2]);
    }
}
