module 0x1::batched_execution {
    struct Foo has drop {}
    struct Bar {}

    struct DroppableValue has drop {
        val: u8,
    }

    struct NonDroppableValue {
        val: u8,
    }

    struct CopyableValue has drop, copy {
        val: u8,
    }

    public fun create_droppable_value(val: u8): DroppableValue {
        DroppableValue { val }
    }

    public fun create_non_droppable_value(val: u8): NonDroppableValue {
        NonDroppableValue { val }
    }

    public fun create_copyable_value(val: u8): CopyableValue {
        CopyableValue { val }
    }

    public fun consume_droppable_value(v: DroppableValue, expected_val: u8) {
        let DroppableValue { val } = v;
        assert!(val == expected_val, 10);
    }

    public fun consume_non_droppable_value(v: NonDroppableValue, expected_val: u8) {
        let NonDroppableValue { val } = v;
        assert!(val == expected_val, 10);
    }

    public fun consume_copyable_value(v: CopyableValue, expected_val: u8) {
        let CopyableValue { val } = v;
        assert!(val == expected_val, 10);
    }

    public fun check_copyable_value(v: &CopyableValue, expected_val: u8) {
        assert!(v.val == expected_val, 10);
    }

    public fun mutate_non_droppable_value(v: &mut NonDroppableValue, new_val: u8) {
        v.val = new_val;
    }

    public fun id<T>(t: T): T {
        t
    }

    struct GenericDroppableValue<phantom T> has drop {
        val: u8
    }

    struct GenericNonDroppableValue<phantom T> {
        val: u8
    }

    public fun create_generic_droppable_value<T>(val: u8): GenericDroppableValue<T> {
        GenericDroppableValue { val }
    }

    public fun create_generic_non_droppable_value<T>(val: u8): GenericNonDroppableValue<T> {
        GenericNonDroppableValue { val }
    }

    public fun consume_generic_non_droppable_value<T>(v: GenericNonDroppableValue<T>, expected_val: u8) {
        let GenericNonDroppableValue { val } = v;
        assert!(val == expected_val, 10);
    }

    public fun multiple_returns(): (DroppableValue, NonDroppableValue) {
        return (DroppableValue { val: 0 }, NonDroppableValue { val: 1} )
    }
}
