spec aptos_framework::optional_aggregator {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: When creating a new integer instance, it guarantees that the limit assigned is a value passed into the
    /// function as an argument, and the value field becomes zero.
    /// Criticality: High
    /// Implementation: The new_integer function sets the limit field to the argument passed in, and the value field is
    /// set to zero.
    /// Enforcement: Formally verified via [high-level-req-1](new_integer).
    ///
    /// No.: 2
    /// Requirement: For a given integer instance it should always be possible to: (1) return the limit value of the integer
    /// resource, (2) return the current value stored in that particular instance, and (3) destroy the integer instance.
    /// Criticality: Low
    /// Implementation: The following functions should not abort if the Integer instance exists: limit(), read_integer(),
    /// destroy_integer().
    /// Enforcement: Formally verified via: [high-level-req-2.1](read_integer), [high-level-req-2.2](limit), and
    /// [high-level-req-2.3](destroy_integer).
    ///
    /// No.: 3
    /// Requirement: Every successful switch must end with the aggregator type changed from non-parallelizable to
    /// parallelizable or vice versa.
    /// Criticality: High
    /// Implementation: The switch function run, if successful, should always change the aggregator type.
    /// Enforcement: Formally verified via [high-level-req-3](switch_and_zero_out).
    /// </high-level-req>
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec OptionalAggregator {
        invariant option::is_some(aggregator) <==> option::is_none(integer);
        invariant option::is_some(integer) <==> option::is_none(aggregator);
        invariant option::is_some(integer) ==> option::borrow(integer).value <= option::borrow(integer).limit;
        invariant option::is_some(aggregator) ==> aggregator::spec_aggregator_get_val(option::borrow(aggregator)) <=
            aggregator::spec_get_limit(option::borrow(aggregator));
    }

    spec new_integer(limit: u128): Integer {
        aborts_if false;
        ensures result.limit == limit;
        /// [high-level-req-1]
        ensures result.value == 0;
    }

    /// Check for overflow.
    spec add_integer(integer: &mut Integer, value: u128) {
        aborts_if value > (integer.limit - integer.value);
        aborts_if integer.value + value > MAX_U128;
        ensures integer.value <= integer.limit;
        ensures integer.value == old(integer.value) + value;
    }

    spec limit {
        /// [high-level-req-2.2]
        aborts_if false;
    }

    spec read_integer {
        /// [high-level-req-2.1]
        aborts_if false;
    }

    spec destroy_integer {
        /// [high-level-req-2.3]
        aborts_if false;
    }

    spec sub(optional_aggregator: &mut OptionalAggregator, value: u128) {
        include SubAbortsIf;
        ensures ((optional_aggregator_value(optional_aggregator) == optional_aggregator_value(old(optional_aggregator)) - value));
    }

    spec schema SubAbortsIf {
        optional_aggregator: OptionalAggregator;
        value: u128;
        aborts_if is_parallelizable(optional_aggregator) && (aggregator::spec_aggregator_get_val(option::borrow(optional_aggregator.aggregator))
            < value);
        aborts_if !is_parallelizable(optional_aggregator) &&
            (option::borrow(optional_aggregator.integer).value < value);
    }

    spec read(optional_aggregator: &OptionalAggregator): u128 {
        ensures !is_parallelizable(optional_aggregator) ==> result == option::borrow(optional_aggregator.integer).value;
        ensures is_parallelizable(optional_aggregator) ==>
            result == aggregator::spec_read(option::borrow(optional_aggregator.aggregator));
    }

    spec add(optional_aggregator: &mut OptionalAggregator, value: u128) {
        include AddAbortsIf;
        ensures ((optional_aggregator_value(optional_aggregator) == optional_aggregator_value(old(optional_aggregator)) + value));
    }

    spec schema AddAbortsIf {
        optional_aggregator: OptionalAggregator;
        value: u128;
        aborts_if is_parallelizable(optional_aggregator) && (aggregator::spec_aggregator_get_val(option::borrow(optional_aggregator.aggregator))
            + value > aggregator::spec_get_limit(option::borrow(optional_aggregator.aggregator)));
        aborts_if is_parallelizable(optional_aggregator) && (aggregator::spec_aggregator_get_val(option::borrow(optional_aggregator.aggregator))
            + value > MAX_U128);
        aborts_if !is_parallelizable(optional_aggregator) &&
            (option::borrow(optional_aggregator.integer).value + value > MAX_U128);
        aborts_if !is_parallelizable(optional_aggregator) &&
            (value > (option::borrow(optional_aggregator.integer).limit - option::borrow(optional_aggregator.integer).value));
    }

    spec switch(_optional_aggregator: &mut OptionalAggregator) {
        aborts_if true;
    }

    spec sub_integer(integer: &mut Integer, value: u128) {
        aborts_if value > integer.value;
        ensures integer.value == old(integer.value) - value;
    }

    spec new(parallelizable: bool): OptionalAggregator {
        aborts_if parallelizable && !exists<aggregator_factory::AggregatorFactory>(@aptos_framework);
        ensures parallelizable ==> is_parallelizable(result);
        ensures !parallelizable ==> !is_parallelizable(result);
        ensures optional_aggregator_value(result) == 0;
        ensures optional_aggregator_value(result) <= optional_aggregator_limit(result);
    }

    spec destroy(optional_aggregator: OptionalAggregator) {
        // aborts_if is_parallelizable(optional_aggregator) && len(optional_aggregator.integer.vec) != 0;
        // aborts_if !is_parallelizable(optional_aggregator) && len(optional_aggregator.integer.vec) == 0;
    }

    /// The aggregator exists and the integer does not exist when destroy the aggregator.
    spec destroy_optional_aggregator(optional_aggregator: OptionalAggregator): u128 {
        // aborts_if len(optional_aggregator.aggregator.vec) == 0;
        // aborts_if len(optional_aggregator.integer.vec) != 0;
        ensures result == aggregator::spec_get_limit(option::borrow(optional_aggregator.aggregator));
    }

    /// The integer exists and the aggregator does not exist when destroy the integer.
    spec destroy_optional_integer(optional_aggregator: OptionalAggregator): u128 {
        // aborts_if len(optional_aggregator.integer.vec) == 0;
        // aborts_if len(optional_aggregator.aggregator.vec) != 0;
        ensures result == option::borrow(optional_aggregator.integer).limit;
    }

    spec fun optional_aggregator_value(optional_aggregator: OptionalAggregator): u128 {
        if (is_parallelizable(optional_aggregator)) {
            aggregator::spec_aggregator_get_val(option::borrow(optional_aggregator.aggregator))
        } else {
            option::borrow(optional_aggregator.integer).value
        }
    }

    spec fun optional_aggregator_limit(optional_aggregator: OptionalAggregator): u128 {
        if (is_parallelizable(optional_aggregator)) {
            aggregator::spec_get_limit(option::borrow(optional_aggregator.aggregator))
        } else {
            option::borrow(optional_aggregator.integer).limit
        }
    }

}
