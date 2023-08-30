module 0x1::aggregator_v2_test {
    use aptos_framework::aggregator_v2;

    public entry fun verify_copy_snapshot() {
        let snapshot = aggregator_v2::create_snapshot(42);
        let snapshot2 = aggregator_v2::copy_snapshot(&snapshot);
        assert!(aggregator_v2::read_snapshot(&snapshot) == 42, 0);
        assert!(aggregator_v2::read_snapshot(&snapshot2) == 42, 0);
    }

    public entry fun verify_copy_string_snapshot() {
        let snapshot = aggregator_v2::create_snapshot(std::string::utf8(b"42"));
        let snapshot2 = aggregator_v2::copy_snapshot(&snapshot);
        assert!(aggregator_v2::read_snapshot(&snapshot) == std::string::utf8(b"42"), 0);
        assert!(aggregator_v2::read_snapshot(&snapshot2) == std::string::utf8(b"42"), 0);
    }

    public entry fun verify_string_concat() {
        let snapshot = aggregator_v2::create_snapshot(42);
        let snapshot2 = aggregator_v2::string_concat(std::string::utf8(b"before"), &snapshot, std::string::utf8(b"after"));
        assert!(aggregator_v2::read_snapshot(&snapshot2) == std::string::utf8(b"before42after"), 0);
    }

    public entry fun verify_string_snapshot_concat() {
        let snapshot = aggregator_v2::create_snapshot(std::string::utf8(b"42"));
        let snapshot2 = aggregator_v2::string_concat(std::string::utf8(b"before"), &snapshot, std::string::utf8(b"after"));
        assert!(aggregator_v2::read_snapshot(&snapshot2) == std::string::utf8(b"before42after"), 0);
    }
}
