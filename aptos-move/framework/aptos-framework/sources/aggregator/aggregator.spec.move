spec aptos_framework::aggregator {
    spec module {
        pragma verify = true;
    }

    spec add(aggregator: &mut Aggregator, value: u128) {
        // TODO: temporary mockup.
        pragma opaque;
        aborts_if false;
        ensures aggregator.limit == old(aggregator.limit);
    }

    spec sub(aggregator: &mut Aggregator, value: u128) {
        // TODO: temporary mockup.
        pragma opaque;
        aborts_if false;
        ensures aggregator.limit == old(aggregator.limit);
    }

    spec read(aggregator: &Aggregator): u128 {
        // TODO: temporary mockup.
        pragma opaque;
        aborts_if false;
        ensures result == spec_read(aggregator);
        ensures result <= aggregator.limit;
    }

    spec destroy(aggregator: Aggregator) {
        pragma opaque;
        aborts_if false;
    }

    spec fun spec_read(aggregator: Aggregator): u128;
}
