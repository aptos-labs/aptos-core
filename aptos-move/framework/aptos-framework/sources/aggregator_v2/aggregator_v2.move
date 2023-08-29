/// This module provides an interface for aggregators (version 2).
/// Only skeleton - for AggregagtorSnapshot - is provided at this time,
/// to allow transition of usages.
module aptos_framework::aggregator_v2 {
    use std::string::String;

    /// The generic type supplied to the aggregator snapshot is not supported
    const EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE: u64 = 5;

    struct AggregatorSnapshot<Element> has store, drop {
        value: Element,
    }

    public native fun create_snapshot<Element: copy + drop>(value: Element): AggregatorSnapshot<Element>;

    public native fun copy_snapshot<Element: copy + drop>(snapshot: &AggregatorSnapshot<Element>): AggregatorSnapshot<Element>;

    public native fun read_snapshot<Element>(snapshot: &AggregatorSnapshot<Element>): Element;

    public native fun string_concat<Element>(before: String, snapshot: &AggregatorSnapshot<Element>, after: String): AggregatorSnapshot<String>;

    #[test(fx = @std)]
    public fun test_correct_read(fx: signer) {
        use std::features;
        let feature = features::get_aggregator_snapshots_feature();
        features::change_feature_flags(&fx, vector[feature], vector[]);

        let snapshot = create_snapshot(42);
        let snapshot2 = copy_snapshot(&snapshot);
        assert!(read_snapshot(&snapshot) == 42, 0);
        assert!(read_snapshot(&snapshot2) == 42, 0);
    }

    #[test(fx = @std)]
    public fun test_correct_read_string(fx: signer) {
        use std::features;
        let feature = features::get_aggregator_snapshots_feature();
        features::change_feature_flags(&fx, vector[feature], vector[]);

        let snapshot = create_snapshot(std::string::utf8(b"42"));
        let snapshot2 = copy_snapshot(&snapshot);
        assert!(read_snapshot(&snapshot) == std::string::utf8(b"42"), 0);
        assert!(read_snapshot(&snapshot2) == std::string::utf8(b"42"), 0);
    }

    #[test(fx = @std)]
    public fun test_string_concat1(fx: signer) {
        use std::features;
        let feature = features::get_aggregator_snapshots_feature();
        features::change_feature_flags(&fx, vector[feature], vector[]);

        let snapshot = create_snapshot(42);
        let snapshot2 = string_concat(std::string::utf8(b"before"), &snapshot, std::string::utf8(b"after"));
        assert!(read_snapshot(&snapshot2) == std::string::utf8(b"before42after"), 0);
    }

    #[test(fx = @std)]
    public fun test_string_concat2(fx: signer) {
        use std::features;
        let feature = features::get_aggregator_snapshots_feature();
        features::change_feature_flags(&fx, vector[feature], vector[]);

        let snapshot = create_snapshot<String>(std::string::utf8(b"42"));
        let snapshot2 = string_concat(std::string::utf8(b"before"), &snapshot, std::string::utf8(b"after"));
        assert!(read_snapshot(&snapshot2) == std::string::utf8(b"before42after"), 0);
    }
    
    #[test]
    #[expected_failure]
    public fun test_snapshot_feature_not_enabled() {
        create_snapshot(42);
    }
}
