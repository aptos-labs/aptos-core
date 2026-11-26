module std::mem {
    /// Swaps the values referenced by `left` and `right`.
    ///
    /// Move's mutable-reference rules require `left` and `right`
    /// to refer to distinct values.
    public native fun swap<T>(left: &mut T, right: &mut T);

    /// Replaces the value referenced by `ref` with `new`
    /// and returns the previous value.
    public fun replace<T>(ref: &mut T, new: T): T {
        swap(ref, &mut new);
        new
    }

    spec swap<T>(left: &mut T, right: &mut T) {
        pragma opaque;
        aborts_if false;
        ensures right == old(left);
        ensures left == old(right);
    }

    spec replace<T>(ref: &mut T, new: T): T {
        pragma opaque;
        aborts_if false;
        ensures result == old(ref);
        ensures ref == new;
    }
}
