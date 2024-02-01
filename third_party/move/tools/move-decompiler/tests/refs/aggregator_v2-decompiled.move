module 0x1::aggregator_v2 {
    struct Aggregator<T0> has drop, store {
        value: T0,
        max_value: T0,
    }
    
    struct AggregatorSnapshot<T0> has drop, store {
        value: T0,
    }
    
    struct DerivedStringSnapshot has drop, store {
        value: 0x1::string::String,
        padding: vector<u8>,
    }
    
    public fun add<T0>(arg0: &mut Aggregator<T0>, arg1: T0) {
        assert!(try_add<T0>(arg0, arg1), 0x1::error::out_of_range(1));
    }
    
    native public fun copy_snapshot<T0: copy + drop>(arg0: &AggregatorSnapshot<T0>) : AggregatorSnapshot<T0>;
    native public fun create_aggregator<T0: copy + drop>(arg0: T0) : Aggregator<T0>;
    native public fun create_derived_string(arg0: 0x1::string::String) : DerivedStringSnapshot;
    native public fun create_snapshot<T0: copy + drop>(arg0: T0) : AggregatorSnapshot<T0>;
    native public fun create_unbounded_aggregator<T0: copy + drop>() : Aggregator<T0>;
    native public fun derive_string_concat<T0>(arg0: 0x1::string::String, arg1: &AggregatorSnapshot<T0>, arg2: 0x1::string::String) : DerivedStringSnapshot;
    public fun max_value<T0: copy + drop>(arg0: &Aggregator<T0>) : T0 {
        arg0.max_value
    }
    
    native public fun read<T0>(arg0: &Aggregator<T0>) : T0;
    native public fun read_derived_string(arg0: &DerivedStringSnapshot) : 0x1::string::String;
    native public fun read_snapshot<T0>(arg0: &AggregatorSnapshot<T0>) : T0;
    native public fun snapshot<T0>(arg0: &Aggregator<T0>) : AggregatorSnapshot<T0>;
    native public fun string_concat<T0>(arg0: 0x1::string::String, arg1: &AggregatorSnapshot<T0>, arg2: 0x1::string::String) : AggregatorSnapshot<0x1::string::String>;
    public fun sub<T0>(arg0: &mut Aggregator<T0>, arg1: T0) {
        assert!(try_sub<T0>(arg0, arg1), 0x1::error::out_of_range(2));
    }
    
    native public fun try_add<T0>(arg0: &mut Aggregator<T0>, arg1: T0) : bool;
    native public fun try_sub<T0>(arg0: &mut Aggregator<T0>, arg1: T0) : bool;
    // decompiled from Move bytecode v6
}
