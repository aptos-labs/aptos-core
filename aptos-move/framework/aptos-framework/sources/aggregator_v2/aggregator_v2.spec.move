spec aptos_framework::aggregator_v2 {

    spec create_aggregator {
        // TODO: temporary mockup.
        pragma opaque;
        aborts_if false;
    }

    spec create_unbounded_aggregator {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec try_add<IntElement>(aggregator: &mut Aggregator<IntElement>, value: IntElement): bool {
        // TODO: temporary mockup.
        pragma opaque;
        aborts_if false;
        ensures spec_aggregator_get_val<IntElement>(aggregator) + value > spec_get_limit<IntElement>(aggregator) ==> result == false;
        ensures spec_aggregator_get_val<IntElement>(aggregator) + value > MAX_U128 ==> result == false;
        ensures spec_aggregator_get_val<IntElement>(aggregator) + value <= spec_get_limit<IntElement>(aggregator)
            && spec_aggregator_get_val<IntElement>(aggregator) + value <= MAX_U128 ==> result == true
            && aggregator == spec_aggregator_set_val<IntElement>(old(aggregator),
            spec_aggregator_get_val<IntElement>(old(aggregator)) + value);
        ensures spec_get_limit<IntElement>(aggregator) == spec_get_limit<IntElement>(old(aggregator));
    }

    spec try_sub<IntElement>(aggregator: &mut Aggregator<IntElement>, value: IntElement): bool {
        // TODO: temporary mockup.
        pragma opaque;
        aborts_if false;
        ensures spec_aggregator_get_val<IntElement>(aggregator) < value ==> result == false;
        ensures spec_aggregator_get_val<IntElement>(aggregator) >= value ==> result == true &&
            aggregator == spec_aggregator_set_val<IntElement>(old(aggregator),
                spec_aggregator_get_val<IntElement>(old(aggregator)) - value);
        ensures spec_get_limit<IntElement>(aggregator) == spec_get_limit<IntElement>(old(aggregator));
    }

    spec read<IntElement>(aggregator: &Aggregator<IntElement>): IntElement {
        // TODO: temporary mockup.
        pragma opaque;
        aborts_if false;
        ensures result == spec_read(aggregator);
        ensures result <= spec_get_limit(aggregator);
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

    spec native fun spec_aggregator_get_val<IntElement>(a: Aggregator<IntElement>): IntElement;
    spec native fun spec_aggregator_set_val<IntElement>(a: Aggregator<IntElement>, v: IntElement): Aggregator<IntElement>;
    spec native fun spec_get_limit<IntElement>(a: Aggregator<IntElement>): IntElement;
    spec native fun spec_read<IntElement>(aggregator: Aggregator<IntElement>): IntElement;
}
