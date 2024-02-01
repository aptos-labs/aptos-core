module 0x1::optional_aggregator {
    struct Integer has store {
        value: u128,
        limit: u128,
    }
    
    struct OptionalAggregator has store {
        aggregator: 0x1::option::Option<0x1::aggregator::Aggregator>,
        integer: 0x1::option::Option<Integer>,
    }
    
    public fun add(arg0: &mut OptionalAggregator, arg1: u128) {
        if (0x1::option::is_some<0x1::aggregator::Aggregator>(&arg0.aggregator)) {
            let v0 = 0x1::option::borrow_mut<0x1::aggregator::Aggregator>(&mut arg0.aggregator);
            0x1::aggregator::add(v0, arg1);
        } else {
            add_integer(0x1::option::borrow_mut<Integer>(&mut arg0.integer), arg1);
        };
    }
    
    public fun destroy(arg0: OptionalAggregator) {
        if (is_parallelizable(&arg0)) {
            destroy_optional_aggregator(arg0);
        } else {
            destroy_optional_integer(arg0);
        };
    }
    
    fun limit(arg0: &Integer) : u128 {
        arg0.limit
    }
    
    public fun read(arg0: &OptionalAggregator) : u128 {
        if (0x1::option::is_some<0x1::aggregator::Aggregator>(&arg0.aggregator)) {
            0x1::aggregator::read(0x1::option::borrow<0x1::aggregator::Aggregator>(&arg0.aggregator))
        } else {
            read_integer(0x1::option::borrow<Integer>(&arg0.integer))
        }
    }
    
    public fun sub(arg0: &mut OptionalAggregator, arg1: u128) {
        if (0x1::option::is_some<0x1::aggregator::Aggregator>(&arg0.aggregator)) {
            let v0 = 0x1::option::borrow_mut<0x1::aggregator::Aggregator>(&mut arg0.aggregator);
            0x1::aggregator::sub(v0, arg1);
        } else {
            sub_integer(0x1::option::borrow_mut<Integer>(&mut arg0.integer), arg1);
        };
    }
    
    fun add_integer(arg0: &mut Integer, arg1: u128) {
        assert!(arg1 <= arg0.limit - arg0.value, 0x1::error::out_of_range(1));
        arg0.value = arg0.value + arg1;
    }
    
    fun destroy_integer(arg0: Integer) {
        let Integer {
            value : _,
            limit : _,
        } = arg0;
    }
    
    fun destroy_optional_aggregator(arg0: OptionalAggregator) : u128 {
        let OptionalAggregator {
            aggregator : v0,
            integer    : v1,
        } = arg0;
        let v2 = v0;
        0x1::aggregator::destroy(0x1::option::destroy_some<0x1::aggregator::Aggregator>(v2));
        0x1::option::destroy_none<Integer>(v1);
        0x1::aggregator::limit(0x1::option::borrow<0x1::aggregator::Aggregator>(&v2))
    }
    
    fun destroy_optional_integer(arg0: OptionalAggregator) : u128 {
        let OptionalAggregator {
            aggregator : v0,
            integer    : v1,
        } = arg0;
        let v2 = v1;
        destroy_integer(0x1::option::destroy_some<Integer>(v2));
        0x1::option::destroy_none<0x1::aggregator::Aggregator>(v0);
        limit(0x1::option::borrow<Integer>(&v2))
    }
    
    public fun is_parallelizable(arg0: &OptionalAggregator) : bool {
        0x1::option::is_some<0x1::aggregator::Aggregator>(&arg0.aggregator)
    }
    
    public(friend) fun new(arg0: u128, arg1: bool) : OptionalAggregator {
        if (arg1) {
            OptionalAggregator{aggregator: 0x1::option::some<0x1::aggregator::Aggregator>(0x1::aggregator_factory::create_aggregator_internal(arg0)), integer: 0x1::option::none<Integer>()}
        } else {
            OptionalAggregator{aggregator: 0x1::option::none<0x1::aggregator::Aggregator>(), integer: 0x1::option::some<Integer>(new_integer(arg0))}
        }
    }
    
    fun new_integer(arg0: u128) : Integer {
        Integer{
            value : 0, 
            limit : arg0,
        }
    }
    
    fun read_integer(arg0: &Integer) : u128 {
        arg0.value
    }
    
    fun sub_integer(arg0: &mut Integer, arg1: u128) {
        assert!(arg1 <= arg0.value, 0x1::error::out_of_range(2));
        arg0.value = arg0.value - arg1;
    }
    
    public fun switch(arg0: &mut OptionalAggregator) {
        switch_and_zero_out(arg0);
        add(arg0, read(arg0));
    }
    
    fun switch_and_zero_out(arg0: &mut OptionalAggregator) {
        if (is_parallelizable(arg0)) {
            switch_to_integer_and_zero_out(arg0);
        } else {
            switch_to_aggregator_and_zero_out(arg0);
        };
    }
    
    fun switch_to_aggregator_and_zero_out(arg0: &mut OptionalAggregator) : u128 {
        let v0 = 0x1::option::extract<Integer>(&mut arg0.integer);
        let v1 = limit(&v0);
        destroy_integer(v0);
        let v2 = 0x1::aggregator_factory::create_aggregator_internal(v1);
        0x1::option::fill<0x1::aggregator::Aggregator>(&mut arg0.aggregator, v2);
        v1
    }
    
    fun switch_to_integer_and_zero_out(arg0: &mut OptionalAggregator) : u128 {
        let v0 = 0x1::option::extract<0x1::aggregator::Aggregator>(&mut arg0.aggregator);
        let v1 = 0x1::aggregator::limit(&v0);
        0x1::aggregator::destroy(v0);
        0x1::option::fill<Integer>(&mut arg0.integer, new_integer(v1));
        v1
    }
    
    // decompiled from Move bytecode v6
}
