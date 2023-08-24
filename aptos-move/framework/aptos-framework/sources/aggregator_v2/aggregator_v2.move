/// This module provides an interface for aggregators (version 2).
/// Only skeleton - for AggregagtorSnapshot - is provided at this time,
/// to allow transition of usages.
module aptos_framework::aggregator_v2 {
    use std::string::String;

    struct AggregatorSnapshot<Element> has store, drop {
        value: Element,
    }

    public native fun create_snapshot<Element: copy + drop>(value: Element): AggregatorSnapshot<Element>;

    public native fun copy_snapshot<Element: copy + drop>(snapshot: &AggregatorSnapshot<Element>): AggregatorSnapshot<Element>;

    public native fun read_snapshot<Element>(snapshot: &AggregatorSnapshot<Element>): Element;

    public native fun string_concat<Element>(before: String, snapshot: &AggregatorSnapshot<Element>, after: String): AggregatorSnapshot<String>;
}