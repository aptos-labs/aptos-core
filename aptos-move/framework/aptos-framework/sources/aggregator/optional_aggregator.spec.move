spec aptos_framework::optional_aggregator {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    /// Check for overflow.
    spec add_integer(integer: &mut Integer, value: u128) {
        aborts_if value > (integer.limit - integer.value);
        aborts_if integer.value + value > MAX_U128;
        ensures integer.value == old(integer.value) + value;
    }

    spec sub(optional_aggregator: &mut OptionalAggregator, value: u128) {
        // TODO: temporary mockup
        pragma opaque, verify = false;
        aborts_if false;
    }

    spec read(optional_aggregator: &OptionalAggregator): u128 {
        pragma verify = false;
    }

    spec add(optional_aggregator: &mut OptionalAggregator, value: u128) {
        pragma verify = false;
    }

    spec switch(optional_aggregator: &mut OptionalAggregator) {
        pragma verify = false;
    }

    spec sub_integer(integer: &mut Integer, value: u128) {
        aborts_if value > integer.value;
        ensures integer.value == old(integer.value) - value;
    }

    spec new(limit: u128, parallelizable: bool): OptionalAggregator {
        aborts_if parallelizable && !exists<aggregator_factory::AggregatorFactory>(@aptos_framework);
    }

    /// Option<Integer> does not exist When Option<Aggregator> exists.
    /// Option<Integer> exists when Option<Aggregator> does not exist.
    /// The AggregatorFactory is under the @aptos_framework when Option<Aggregator> does not exist.
    spec switch_and_zero_out(optional_aggregator: &mut OptionalAggregator) {
        let vec_ref = optional_aggregator.integer.vec;
        aborts_if is_parallelizable(optional_aggregator) && len(vec_ref) != 0;
        aborts_if !is_parallelizable(optional_aggregator) && len(vec_ref) == 0;
        aborts_if !is_parallelizable(optional_aggregator) && !exists<aggregator_factory::AggregatorFactory>(@aptos_framework);
    }

    /// The aggregator exists and the integer dosex not exist when Switches from parallelizable to non-parallelizable implementation.
    spec switch_to_integer_and_zero_out(
        optional_aggregator: &mut OptionalAggregator
    ): u128 {
        aborts_if len(optional_aggregator.aggregator.vec) == 0;
        aborts_if len(optional_aggregator.integer.vec) != 0;
    }

    /// The integer exists and the aggregator does not exist when Switches from non-parallelizable to parallelizable implementation.
    /// The AggregatorFactory is under the @aptos_framework.
    spec switch_to_aggregator_and_zero_out(
        optional_aggregator: &mut OptionalAggregator
    ): u128 {
        aborts_if len(optional_aggregator.integer.vec) == 0;
        aborts_if !exists<aggregator_factory::AggregatorFactory>(@aptos_framework);
        aborts_if len(optional_aggregator.aggregator.vec) != 0;
    }

    spec destroy(optional_aggregator: OptionalAggregator) {
        aborts_if is_parallelizable(optional_aggregator) && len(optional_aggregator.integer.vec) != 0;
        aborts_if !is_parallelizable(optional_aggregator) && len(optional_aggregator.integer.vec) == 0;
    }

    /// The aggregator exists and the integer does not exist when destroy the aggregator.
    spec destroy_optional_aggregator(optional_aggregator: OptionalAggregator): u128 {
        aborts_if len(optional_aggregator.aggregator.vec) == 0;
        aborts_if len(optional_aggregator.integer.vec) != 0;
    }

    /// The integer exists and the aggregator does not exist when destroy the integer.
    spec destroy_optional_integer(optional_aggregator: OptionalAggregator): u128 {
        aborts_if len(optional_aggregator.integer.vec) == 0;
        aborts_if len(optional_aggregator.aggregator.vec) != 0;
    }
}
