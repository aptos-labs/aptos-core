/// Module with methods for safe memory manipulation.
/// I.e. swapping/replacing non-copyable/non-droppable types.
module std::mem {
    /// Swap contents of two passed mutable references.
    public native fun swap<T>(left: &mut T, right: &mut T);

    /// Replace value reference points to with the given new value,
    /// and return value it had before.
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
