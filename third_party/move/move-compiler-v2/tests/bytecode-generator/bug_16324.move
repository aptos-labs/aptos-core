module 0xc0ffee::m {
    use std::vector;

    struct TestOrder has store, copy, drop {
        price: u64,
        size: u64,
    }

    fun price(self: &TestOrder): u64 {
        self.price
    }

    fun size(self: &TestOrder): u64 {
        self.size
    }

    fun test_self(orders: vector<TestOrder>) {

        vector::for_each_ref(
            &orders,
            |order| {
                let _price = order.price();
                let _size = order.size();
            }
        );

    }
}
