spec aptos_framework::object {
    spec exists_at {
        pragma opaque;
        aborts_if false;
        // TODO: Disabled the following post-condition due to an issue with
        // the use of a type parameter in `exists` in spec.
        // ensures result == exists<T>(object);
    }
}
