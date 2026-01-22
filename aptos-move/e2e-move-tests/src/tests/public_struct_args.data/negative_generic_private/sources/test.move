/// Test module with generic entry function using private struct as type argument.
/// This should fail at runtime validation because Container<PrivatePoint> contains
/// a private field type when T is instantiated with PrivatePoint.
module 0xcafe::negative_generic_private {

    /// A public generic container struct with copy ability
    public struct Container<T> has copy, drop {
        value: T,
    }

    /// Private struct (not public) - should not be allowed as type argument
    struct PrivatePoint has copy, drop {
        x: u64,
        y: u64,
    }

    /// Generic entry function that takes Container<T> for any T: copy + drop
    /// When called with Container<PrivatePoint>, this should fail at runtime validation
    /// because PrivatePoint is private and cannot be used as a transaction argument,
    /// even though Container is public and has copy ability.
    public entry fun test_generic_container<T: copy + drop>(_sender: &signer, _container: Container<T>) {
        // This should never execute when T = PrivatePoint
    }
}
