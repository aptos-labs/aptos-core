/// This module provides an interface for aggregators (version 2).
module aptos_framework::aggregator_v2 {
    use std::error;

    /// The value of aggregator overflows. Raised by uncoditional add() call
    const EAGGREGATOR_OVERFLOW: u64 = 1;

    /// The value of aggregator underflows (goes below zero). Raised by uncoditional sub() call
    const EAGGREGATOR_UNDERFLOW: u64 = 2;

    /// Element type of the aggregator not supported. Raised by create_aggregator() call,
    /// if Element type is not supported (currently u64 or u128).
    const EAGGREGATOR_ELEMENT_TYPE_NOT_SUPPORTED: u64 = 3;

    /// Represents an integer which supports parallel additions and subtractions
    /// across multiple transactions. See the module description for more details.
    ///
    /// Currently supported types for Element are u64 and u128.
    struct Aggregator<Element> has store {
        value: Element,
        max_value: Element,
    }

    struct AggregatorSnapshot<Element> has store {
        value: Element,
    }

    /// Returns `max_value` exceeding which aggregator overflows.
    public fun max_value<Element: copy + drop>(aggregator: &Aggregator<Element>): Element {
        aggregator.max_value
    }

    /// Creates new aggregator, with given 'max_value'.
    ///
    /// Currently supported types for Element are u64 and u128.
    /// EAGGREGATOR_ELEMENT_TYPE_NOT_SUPPORTED raised if called with a different type.
    public native fun create_aggregator<Element: copy + drop>(max_value: Element): Aggregator<Element>;

    /// Adds `value` to aggregator.
    /// If addition would exceed the max_value, `false` is returned, and aggregator value is left unchanged.
    public native fun try_add<Element>(aggregator: &mut Aggregator<Element>, value: Element): bool;

    // Adds `value` to aggregator, uncoditionally.
    // If addition would exceed the max_value, EAGGREGATOR_OVERFLOW exception will be thrown
    public fun add<Element>(aggregator: &mut Aggregator<Element>, value: Element) {
        assert!(try_add(aggregator, value), error::out_of_range(EAGGREGATOR_OVERFLOW));
    }

    /// Subtracts `value` from aggregator.
    /// If subtraction would result in a negative value, `false` is returned, and aggregator value is left unchanged.
    public native fun try_sub<Element>(aggregator: &mut Aggregator<Element>, value: Element): bool;

    // Adds `value` to aggregator, uncoditionally.
    // If subtraction would result in a negative value, EAGGREGATOR_UNDERFLOW exception will be thrown
    public fun sub<Element>(aggregator: &mut Aggregator<Element>, value: Element) {
        assert!(try_sub(aggregator, value), error::out_of_range(EAGGREGATOR_UNDERFLOW));
    }

    /// Returns a value stored in this aggregator.
    public native fun read<Element>(aggregator: &Aggregator<Element>): Element;

    public native fun snapshot<Element>(aggregator: &Aggregator<Element>): AggregatorSnapshot<Element>;

    public native fun read_snapshot<Element>(snapshot: &AggregatorSnapshot<Element>): Element;
}
