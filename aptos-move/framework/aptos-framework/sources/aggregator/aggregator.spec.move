spec aptos_framework::aggregator {
    spec module {
        pragma verify = true;
    }

    spec add(aggregator: &mut Aggregator, value: u128) {
        // TODO: temporary mockup.
        pragma opaque;
        aborts_if !spec_aggreator_exists(aggregator);
    }

    spec sub(aggregator: &mut Aggregator, value: u128) {
        // TODO: temporary mockup.
        pragma opaque;
        aborts_if !spec_aggreator_exists(aggregator);
    }

    spec read(aggregator: &Aggregator): u128 {
        // TODO: temporary mockup.
        pragma opaque;
        aborts_if !spec_aggreator_exists(aggregator);
    }

    spec destroy(aggregator: Aggregator) {
        // TODO: temporary mockup.
        pragma opaque;
        // TODO: this aborts_if condition is a temporary mockup, which needs to be refined.
        aborts_if false;
    }

    spec fun spec_aggreator_exists(aggregator: Aggregator): bool;
}
