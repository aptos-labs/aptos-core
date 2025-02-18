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
        invariant aggregator.is_some() <==> integer.is_none();
        invariant integer.is_some() <==> aggregator.is_none();
        invariant integer.is_some() ==> integer.borrow().value <= integer.borrow().limit;
        invariant aggregator.is_some() ==> aggregator::spec_aggregator_get_val(aggregator.borrow()) <=
            aggregator::spec_get_limit(aggregator.borrow());
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
        aborts_if is_parallelizable(optional_aggregator) && (aggregator::spec_aggregator_get_val(
            optional_aggregator.aggregator.borrow()
        )
            < value);
        aborts_if !is_parallelizable(optional_aggregator) &&
            (optional_aggregator.integer.borrow().value < value);
    }

    spec read(optional_aggregator: &OptionalAggregator): u128 {
        ensures !is_parallelizable(optional_aggregator) ==> result == optional_aggregator.integer.borrow().value;
        ensures is_parallelizable(optional_aggregator) ==>
            result == aggregator::spec_read(optional_aggregator.aggregator.borrow());
    }

    spec add(optional_aggregator: &mut OptionalAggregator, value: u128) {
        include AddAbortsIf;
        ensures ((optional_aggregator_value(optional_aggregator) == optional_aggregator_value(old(optional_aggregator)) + value));
    }

    spec schema AddAbortsIf {
        optional_aggregator: OptionalAggregator;
        value: u128;
        aborts_if is_parallelizable(optional_aggregator) && (aggregator::spec_aggregator_get_val(
            optional_aggregator.aggregator.borrow()
        )
            + value > aggregator::spec_get_limit(optional_aggregator.aggregator.borrow()));
        aborts_if is_parallelizable(optional_aggregator) && (aggregator::spec_aggregator_get_val(
            optional_aggregator.aggregator.borrow()
        )
            + value > MAX_U128);
        aborts_if !is_parallelizable(optional_aggregator) &&
            (optional_aggregator.integer.borrow().value + value > MAX_U128);
        aborts_if !is_parallelizable(optional_aggregator) &&
            (value > (optional_aggregator.integer.borrow().limit - optional_aggregator.integer.borrow().value));
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
        aborts_if is_parallelizable(optional_aggregator) && len(optional_aggregator.integer.vec) != 0;
        aborts_if !is_parallelizable(optional_aggregator) && len(optional_aggregator.integer.vec) == 0;
    }

    /// The aggregator exists and the integer does not exist when destroy the aggregator.
    spec destroy_optional_aggregator(optional_aggregator: OptionalAggregator): u128 {
        aborts_if len(optional_aggregator.aggregator.vec) == 0;
        aborts_if len(optional_aggregator.integer.vec) != 0;
        ensures result == aggregator::spec_get_limit(optional_aggregator.aggregator.borrow());
    }

    /// The integer exists and the aggregator does not exist when destroy the integer.
    spec destroy_optional_integer(optional_aggregator: OptionalAggregator): u128 {
        aborts_if len(optional_aggregator.integer.vec) == 0;
        aborts_if len(optional_aggregator.aggregator.vec) != 0;
        ensures result == optional_aggregator.integer.borrow().limit;
    }

    spec fun optional_aggregator_value(optional_aggregator: OptionalAggregator): u128 {
        if (is_parallelizable(optional_aggregator)) {
            aggregator::spec_aggregator_get_val(optional_aggregator.aggregator.borrow())
        } else {
            optional_aggregator.integer.borrow().value
        }
    }

    spec fun optional_aggregator_limit(optional_aggregator: OptionalAggregator): u128 {
        if (is_parallelizable(optional_aggregator)) {
            aggregator::spec_get_limit(optional_aggregator.aggregator.borrow())
        } else {
            optional_aggregator.integer.borrow().limit
        }
    }

}
