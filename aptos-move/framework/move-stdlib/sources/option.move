/// This module defines the Option type and its methods to represent and handle an optional value.
module std::option {
    use std::vector;

    /// Abstraction of a value that may or may not be present. Implemented with a vector of size
    /// zero or one because Move bytecode does not have ADTs.
    struct Option<Element> has copy, drop, store {
        vec: vector<Element>
    }
    spec Option {
        /// The size of vector is always less than equal to 1
        /// because it's 0 for "none" or 1 for "some".
        invariant len(vec) <= 1;
    }

    /// The `Option` is in an invalid state for the operation attempted.
    /// The `Option` is `Some` while it should be `None`.
    const EOPTION_IS_SET: u64 = 0x40000;
    /// The `Option` is in an invalid state for the operation attempted.
    /// The `Option` is `None` while it should be `Some`.
    const EOPTION_NOT_SET: u64 = 0x40001;
    /// Cannot construct an option from a vector with 2 or more elements.
    const EOPTION_VEC_TOO_LONG: u64 = 0x40002;

    /// Return an empty `Option`
    public fun none<Element>(): Option<Element> {
        Option { vec: vector::empty() }
    }
    spec none {
        pragma opaque;
        aborts_if false;
        ensures result == spec_none<Element>();
    }
    spec fun spec_none<Element>(): Option<Element> {
        Option{ vec: vec() }
    }

    /// Return an `Option` containing `e`
    public fun some<Element>(e: Element): Option<Element> {
        Option { vec: vector::singleton(e) }
    }
    spec some {
        pragma opaque;
        aborts_if false;
        ensures result == spec_some(e);
    }
    spec fun spec_some<Element>(e: Element): Option<Element> {
        Option{ vec: vec(e) }
    }

    public fun from_vec<Element>(vec: vector<Element>): Option<Element> {
        assert!(vector::length(&vec) <= 1, EOPTION_VEC_TOO_LONG);
        Option { vec }
    }

    spec from_vec {
        aborts_if vector::length(vec) > 1;
    }

    /// Return true if `self` does not hold a value
    public fun is_none<Element>(self: &Option<Element>): bool {
        vector::is_empty(&self.vec)
    }
    spec is_none {
        pragma opaque;
        aborts_if false;
        ensures result == spec_is_none(self);
    }
    spec fun spec_is_none<Element>(self: Option<Element>): bool {
        vector::is_empty(self.vec)
    }

    /// Return true if `self` holds a value
    public fun is_some<Element>(self: &Option<Element>): bool {
        !vector::is_empty(&self.vec)
    }
    spec is_some {
        pragma opaque;
        aborts_if false;
        ensures result == spec_is_some(self);
    }
    spec fun spec_is_some<Element>(self: Option<Element>): bool {
        !vector::is_empty(self.vec)
    }

    /// Return true if the value in `self` is equal to `e_ref`
    /// Always returns `false` if `self` does not hold a value
    public fun contains<Element>(self: &Option<Element>, e_ref: &Element): bool {
        vector::contains(&self.vec, e_ref)
    }
    spec contains {
        pragma opaque;
        aborts_if false;
        ensures result == spec_contains(self, e_ref);
    }
    spec fun spec_contains<Element>(self: Option<Element>, e: Element): bool {
        is_some(self) && borrow(self) == e
    }

    /// Return an immutable reference to the value inside `self`
    /// Aborts if `self` does not hold a value
    public fun borrow<Element>(self: &Option<Element>): &Element {
        assert!(is_some(self), EOPTION_NOT_SET);
        vector::borrow(&self.vec, 0)
    }
    spec borrow {
        pragma opaque;
        include AbortsIfNone<Element>;
        ensures result == spec_borrow(self);
    }
    spec fun spec_borrow<Element>(self: Option<Element>): Element {
        self.vec[0]
    }

    /// Return a reference to the value inside `self` if it holds one
    /// Return `default_ref` if `self` does not hold a value
    public fun borrow_with_default<Element>(self: &Option<Element>, default_ref: &Element): &Element {
        let vec_ref = &self.vec;
        if (vector::is_empty(vec_ref)) default_ref
        else vector::borrow(vec_ref, 0)
    }
    spec borrow_with_default {
        pragma opaque;
        aborts_if false;
        ensures result == (if (spec_is_some(self)) spec_borrow(self) else default_ref);
    }

    /// Return the value inside `self` if it holds one
    /// Return `default` if `self` does not hold a value
    public fun get_with_default<Element: copy + drop>(
        self: &Option<Element>,
        default: Element,
    ): Element {
        let vec_ref = &self.vec;
        if (vector::is_empty(vec_ref)) default
        else *vector::borrow(vec_ref, 0)
    }
    spec get_with_default {
        pragma opaque;
        aborts_if false;
        ensures result == (if (spec_is_some(self)) spec_borrow(self) else default);
    }

    /// Convert the none option `self` to a some option by adding `e`.
    /// Aborts if `self` already holds a value
    public fun fill<Element>(self: &mut Option<Element>, e: Element) {
        let vec_ref = &mut self.vec;
        if (vector::is_empty(vec_ref)) vector::push_back(vec_ref, e)
        else abort EOPTION_IS_SET
    }
    spec fill {
        pragma opaque;
        aborts_if spec_is_some(self) with EOPTION_IS_SET;
        ensures spec_is_some(self);
        ensures spec_borrow(self) == e;
    }

    /// Convert a `some` option to a `none` by removing and returning the value stored inside `self`
    /// Aborts if `self` does not hold a value
    public fun extract<Element>(self: &mut Option<Element>): Element {
        assert!(is_some(self), EOPTION_NOT_SET);
        vector::pop_back(&mut self.vec)
    }
    spec extract {
        pragma opaque;
        include AbortsIfNone<Element>;
        ensures result == spec_borrow(old(self));
        ensures spec_is_none(self);
    }

    /// Return a mutable reference to the value inside `self`
    /// Aborts if `self` does not hold a value
    public fun borrow_mut<Element>(self: &mut Option<Element>): &mut Element {
        assert!(is_some(self), EOPTION_NOT_SET);
        vector::borrow_mut(&mut self.vec, 0)
    }
    spec borrow_mut {
        include AbortsIfNone<Element>;
        ensures result == spec_borrow(self);
        ensures self == old(self);
    }

    /// Swap the old value inside `self` with `e` and return the old value
    /// Aborts if `self` does not hold a value
    public fun swap<Element>(self: &mut Option<Element>, e: Element): Element {
        assert!(is_some(self), EOPTION_NOT_SET);
        let vec_ref = &mut self.vec;
        let old_value = vector::pop_back(vec_ref);
        vector::push_back(vec_ref, e);
        old_value
    }
    spec swap {
        pragma opaque;
        include AbortsIfNone<Element>;
        ensures result == spec_borrow(old(self));
        ensures spec_is_some(self);
        ensures spec_borrow(self) == e;
    }

    /// Swap the old value inside `self` with `e` and return the old value;
    /// or if there is no old value, fill it with `e`.
    /// Different from swap(), swap_or_fill() allows for `self` not holding a value.
    public fun swap_or_fill<Element>(self: &mut Option<Element>, e: Element): Option<Element> {
        let vec_ref = &mut self.vec;
        let old_value = if (vector::is_empty(vec_ref)) none()
            else some(vector::pop_back(vec_ref));
        vector::push_back(vec_ref, e);
        old_value
    }
    spec swap_or_fill {
        pragma opaque;
        aborts_if false;
        ensures result == old(self);
        ensures spec_borrow(self) == e;
    }

    /// Destroys `self.` If `self` holds a value, return it. Returns `default` otherwise
    public fun destroy_with_default<Element: drop>(self: Option<Element>, default: Element): Element {
        let Option { vec } = self;
        if (vector::is_empty(&mut vec)) default
        else vector::pop_back(&mut vec)
    }
    spec destroy_with_default {
        pragma opaque;
        aborts_if false;
        ensures result == (if (spec_is_some(self)) spec_borrow(self) else default);
    }

    /// Unpack `self` and return its contents
    /// Aborts if `self` does not hold a value
    public fun destroy_some<Element>(self: Option<Element>): Element {
        assert!(is_some(&self), EOPTION_NOT_SET);
        let Option { vec } = self;
        let elem = vector::pop_back(&mut vec);
        vector::destroy_empty(vec);
        elem
    }
    spec destroy_some {
        pragma opaque;
        include AbortsIfNone<Element>;
        ensures result == spec_borrow(self);
    }

    /// Unpack `self`
    /// Aborts if `self` holds a value
    public fun destroy_none<Element>(self: Option<Element>) {
        assert!(is_none(&self), EOPTION_IS_SET);
        let Option { vec } = self;
        vector::destroy_empty(vec)
    }
    spec destroy_none {
        pragma opaque;
        aborts_if spec_is_some(self) with EOPTION_IS_SET;
    }

    /// Convert `self` into a vector of length 1 if it is `Some`,
    /// and an empty vector otherwise
    public fun to_vec<Element>(self: Option<Element>): vector<Element> {
        let Option { vec } = self;
        vec
    }
    spec to_vec {
        pragma opaque;
        aborts_if false;
        ensures result == self.vec;
    }
    /// Apply the function to the optional element, consuming it. Does nothing if no value present.
    public inline fun for_each<Element>(self: Option<Element>, f: |Element|) {
        if (is_some(&self)) {
            f(destroy_some(self))
        } else {
            destroy_none(self)
        }
    }

    /// Apply the function to the optional element reference. Does nothing if no value present.
    public inline fun for_each_ref<Element>(self: &Option<Element>, f: |&Element|) {
        if (is_some(self)) {
            f(borrow(self))
        }
    }

    /// Apply the function to the optional element reference. Does nothing if no value present.
    public inline fun for_each_mut<Element>(self: &mut Option<Element>, f: |&mut Element|) {
        if (is_some(self)) {
            f(borrow_mut(self))
        }
    }

    /// Folds the function over the optional element.
    public inline fun fold<Accumulator, Element>(
        self: Option<Element>,
        init: Accumulator,
        f: |Accumulator,Element|Accumulator
    ): Accumulator {
        if (is_some(&self)) {
            f(init, destroy_some(self))
        } else {
            destroy_none(self);
            init
        }
    }

    /// Maps the content of an option.
    public inline fun map<Element, OtherElement>(self: Option<Element>, f: |Element|OtherElement): Option<OtherElement> {
        if (is_some(&self)) {
            some(f(destroy_some(self)))
        } else {
            destroy_none(self);
            none()
        }
    }

    /// Maps the content of an option without destroying the original option.
    public inline fun map_ref<Element, OtherElement>(
        self: &Option<Element>, f: |&Element|OtherElement): Option<OtherElement> {
        if (is_some(self)) {
            some(f(borrow(self)))
        } else {
            none()
        }
    }

    /// Filters the content of an option
    public inline fun filter<Element:drop>(self: Option<Element>, f: |&Element|bool): Option<Element> {
        if (is_some(&self) && f(borrow(&self))) {
            self
        } else {
            none()
        }
    }

    /// Returns true if the option contains an element which satisfies predicate.
    public inline fun any<Element>(self: &Option<Element>, p: |&Element|bool): bool {
        is_some(self) && p(borrow(self))
    }

    /// Utility function to destroy an option that is not droppable.
    public inline fun destroy<Element>(self: Option<Element>, d: |Element|) {
        let vec = to_vec(self);
        vector::destroy(vec, |e| d(e));
    }

    spec module {} // switch documentation context back to module level

    spec module {
        pragma aborts_if_is_strict;
    }

    /// # Helper Schema

    spec schema AbortsIfNone<Element> {
        self: Option<Element>;
        aborts_if spec_is_none(self) with EOPTION_NOT_SET;
    }
}
