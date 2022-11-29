spec aptos_framework::aggregator {
    spec module {
        pragma verify = true;
    }

    spec add {
        // TODO: temporary mockup.
        pragma opaque;
        aborts_if aggregator.limit + value > MAX_U128;
    }

    spec sub {
        // TODO: temporary mockup.
        pragma opaque;
        aborts_if aggregator.limit - value < 0;
    }

    spec read {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec destroy {
        // TODO: temporary mockup.
        pragma opaque;
        aborts_if false;
    }
}
