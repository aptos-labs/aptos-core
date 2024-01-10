spec aptos_framework::aggregator_v2 {
    use aptos_framework::aggregator;
    spec module {
        pragma verify = true;
    }

    spec create_aggregator {
        // TODO: temporary mockup.
        pragma opaque;
        aborts_if false;
    }

    spec create_unbounded_aggregator {
        // TODO: temporary mockup.
        pragma opaque;
    }


    spec try_add<IntElement>(aggregator: Aggregator<IntElement>, value: IntElement) {
        // TODO: temporary mockup.
        pragma opaque;
        aborts_if false;
        ensures aggregator::spec_aggregator_get_val(aggregator) + value > aggregator::spec_get_limit(aggregator)
            ==> result == false;
        ensures aggregator::spec_aggregator_get_val(aggregator) + value > MAX_U128 ==> result == false;
        ensures aggregator::spec_aggregator_get_val(aggregator) + value <= aggregator::spec_get_limit(aggregator)
            && aggregator::spec_aggregator_get_val(aggregator) + value <= MAX_U128
            ==> result == true && aggregator == aggregator::spec_aggregator_set_val(old(aggregator),
            aggregator::spec_aggregator_get_val(old(aggregator)) + value);
        ensures aggregator::spec_get_limit(aggregator) == aggregator::spec_get_limit(old(aggregator));
    }

    spec try_sub {
        // TODO: temporary mockup.
        pragma opaque;
        aborts_if false;
        ensures aggregator::spec_aggregator_get_val(aggregator) < value ==> result == false;
        ensures aggregator::spec_aggregator_get_val(aggregator) >= value ==> result == true &&
            aggregator == aggregator::spec_aggregator_set_val(old(aggregator),
                aggregator::spec_aggregator_get_val(old(aggregator)) - value);
        ensures aggregator::spec_get_limit(aggregator) == aggregator::spec_get_limit(old(aggregator));
    }

    spec read {
        // TODO: temporary mockup.
        pragma opaque;
        aborts_if false;
        ensures result == aggregator::spec_read(aggregator);
        ensures result <= aggregator::spec_get_limit(aggregator);
    }

    spec snapshot {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec create_snapshot {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec copy_snapshot {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec string_concat {
        // TODO: temporary mockup.
        pragma opaque;
    }
}
