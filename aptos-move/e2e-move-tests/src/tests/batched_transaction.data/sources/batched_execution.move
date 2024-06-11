module 0x1::batched_execution {
    struct DroppableValue has drop {
        val: u8,
    }

    struct NonDroppableValue {
        val: u8,
    }

    struct CopyableValue has copy {
        val: u8,
    }

    public fun create_droppable_value_with_signer(_s: &signer, val: u8): DroppableValue {
        DroppableValue { val }
    }

    public fun create_non_droppable_value_with_signer(_s: &signer, val: u8): NonDroppableValue {
        NonDroppableValue { val }
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

    public fun consume_droppable_value_with_signer(_s: &signer, v: DroppableValue, expected_val: u8) {
        let DroppableValue { val } = v;
        assert!(val == expected_val, 10);
    }

    public fun consume_non_droppable_value_with_signer(_s: &signer, v: NonDroppableValue, expected_val: u8) {
        let NonDroppableValue { val } = v;
        assert!(val == expected_val, 10);
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
}
