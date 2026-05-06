//# publish
module 0x42::ref_constraint_test {
    struct Container<V> has store, drop {
        v: V
    }

    struct Item has store, copy, drop {
        size: u64
    }

    fun set_size(self: &mut Item, new_size: u64) {
        self.size = new_size;
    }

    inline fun modify_and_return<V: store, R>(self: &mut Container<V>, f: |&mut V|R): R {
        f(&mut self.v)
    }

    // Exercises SomeReference + SomeReceiverFunction coexistence: in the
    // lambda body, `*item` adds a SomeReference constraint while
    // `item.set_size(size)` adds a SomeReceiverFunction constraint on the
    // same fresh type variable. Compilation requires the two to be treated
    // as orthogonal.
    fun call_receiver(c: &mut Container<Item>, size: u64): Item {
        c.modify_and_return(
            |item| {
                item.set_size(size);
                *item
            }
        )
    }

    // Exercises SomeReference + SomeStruct coexistence: `*item` adds a
    // SomeReference constraint while `item.size = ...` adds a SomeStruct
    // constraint on the same fresh type variable.
    fun call_field(c: &mut Container<Item>, new_size: u64): Item {
        c.modify_and_return(
            |item| {
                item.size = new_size;
                *item
            }
        )
    }

    fun test_receiver() {
        let c = Container { v: Item { size: 1 } };
        let result = call_receiver(&mut c, 42);
        assert!(result.size == 42, 0);
        assert!(c.v.size == 42, 1);
    }

    fun test_field() {
        let c = Container { v: Item { size: 1 } };
        let result = call_field(&mut c, 7);
        assert!(result.size == 7, 0);
        assert!(c.v.size == 7, 1);
    }
}

//# run --verbose -- 0x42::ref_constraint_test::test_receiver

//# run --verbose -- 0x42::ref_constraint_test::test_field
