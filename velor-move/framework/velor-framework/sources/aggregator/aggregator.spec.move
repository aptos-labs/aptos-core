spec velor_framework::aggregator {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: For a given aggregator, it should always be possible to: Return the limit value of the aggregator.
    /// Return the current value stored in the aggregator. Destroy an aggregator, removing it from its
    /// AggregatorFactory.
    /// Criticality: Low
    /// Implementation: The following functions should not abort if EventHandle exists: limit(), read(), destroy().
    /// Enforcement: Formally verified via [high-level-req-1.1](read), [high-level-req-1.2](destroy), and [high-level-req-1.3](limit).
    ///
    /// No.: 2
    /// Requirement: If the value during addition exceeds the limit, an overflow occurs.
    /// Criticality: High
    /// Implementation: The native add() function checks the value of the addition to ensure it does not pass the
    /// defined limit and results in aggregator overflow.
    /// Enforcement: Formally verified via [high-level-req-2](add).
    ///
    /// No.: 3
    /// Requirement: Operations over aggregators should be correct.
    /// Criticality: High
    /// Implementation: The implementation of the add, sub, read and destroy functions is correct.
    /// Enforcement: The native implementation of the add, sub, read and destroy functions have been manually audited.
    /// </high-level-req>
    ///
    spec Aggregator {
        pragma intrinsic;
    }

    spec add(aggregator: &mut Aggregator, value: u128) {
        pragma opaque;
        aborts_if spec_aggregator_get_val(aggregator) + value > spec_get_limit(aggregator);
        /// [high-level-req-2]
        aborts_if spec_aggregator_get_val(aggregator) + value > MAX_U128;
        ensures spec_get_limit(aggregator) == spec_get_limit(old(aggregator));
        ensures aggregator == spec_aggregator_set_val(old(aggregator),
            spec_aggregator_get_val(old(aggregator)) + value);
    }

    spec sub(aggregator: &mut Aggregator, value: u128) {
        pragma opaque;
        aborts_if spec_aggregator_get_val(aggregator) < value;
        ensures spec_get_limit(aggregator) == spec_get_limit(old(aggregator));
        ensures aggregator == spec_aggregator_set_val(old(aggregator),
            spec_aggregator_get_val(old(aggregator)) - value);
    }

    spec read(aggregator: &Aggregator): u128 {
        pragma opaque;
        /// [high-level-req-1.1]
        aborts_if false;
        ensures result == spec_read(aggregator);
        ensures result <= spec_get_limit(aggregator);
    }

    spec destroy(aggregator: Aggregator) {
        pragma opaque;
        /// [high-level-req-1.2]
        aborts_if false;
    }

    spec limit {
        pragma intrinsic;
        /// [high-level-req-1.2]
        aborts_if [abstract] false;
    }

    spec native fun spec_read(aggregator: Aggregator): u128;
    spec native fun spec_get_limit(a: Aggregator): u128;
    spec native fun spec_get_handle(a: Aggregator): u128;
    spec native fun spec_get_key(a: Aggregator): u128;
    spec native fun spec_aggregator_set_val(a: Aggregator, v: u128): Aggregator;
    spec native fun spec_aggregator_get_val(a: Aggregator): u128;
}
