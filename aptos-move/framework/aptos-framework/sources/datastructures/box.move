module aptos_framework::box {
    use std::signer;
    use aptos_framework::transaction_context;

    struct BoxedResource<T> has key {
        val: T
    }

    struct Box<phantom T> has store {
        addr: address
    }

    public fun new<T: store>(value: T): Box<T> {
        let unique_signer = transaction_context::generate_unique_signer();
        move_to(&unique_signer, BoxedResource { val: value });
        Box { addr: signer::address_of(&unique_signer) }
    }

    // Internal natives that take BoxedResource<T> as a type parameter (like table's borrow_box)
    native fun borrow_boxed<T: store, BR>(self: &Box<T>): &BR;
    native fun borrow_boxed_mut<T: store, BR>(self: &mut Box<T>): &mut BR;

    public fun borrow<T: store>(self: &Box<T>): &T {
        &self.borrow_boxed<T, BoxedResource<T>>().val
    }

    public fun borrow_mut<T: store>(self: &mut Box<T>): &mut T {
        &mut self.borrow_boxed_mut<T, BoxedResource<T>>().val
    }

    public fun copy_box<T: store + copy>(self: &Box<T>): Box<T> {
        new(*self.borrow())
    }

    public fun destroy<T: store>(self: Box<T>): T {
        let Box { addr } = self;
        let BoxedResource { val } = move_from<BoxedResource<T>>(addr);
        val
    }

    #[test]
    public fun test_box() {
        let box = new(1u64);
        assert!(box.borrow() == &1);
        *box.borrow_mut() += 1;
        assert!(box.borrow() == &2);
        assert!(box.destroy() == 2);
    }
}
