module 0x1::aggregator_factory {
    struct AggregatorFactory has key {
        phantom_table: 0x1::table::Table<address, u128>,
    }
    
    public fun create_aggregator(arg0: &signer, arg1: u128) : 0x1::aggregator::Aggregator acquires AggregatorFactory {
        0x1::system_addresses::assert_aptos_framework(arg0);
        create_aggregator_internal(arg1)
    }
    
    public(friend) fun create_aggregator_internal(arg0: u128) : 0x1::aggregator::Aggregator acquires AggregatorFactory {
        assert!(exists<AggregatorFactory>(@0x1), 0x1::error::not_found(1));
        new_aggregator(borrow_global_mut<AggregatorFactory>(@0x1), arg0)
    }
    
    public(friend) fun initialize_aggregator_factory(arg0: &signer) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = AggregatorFactory{phantom_table: 0x1::table::new<address, u128>()};
        move_to<AggregatorFactory>(arg0, v0);
    }
    
    native fun new_aggregator(arg0: &mut AggregatorFactory, arg1: u128) : 0x1::aggregator::Aggregator;
    // decompiled from Move bytecode v6
}
