spec aptos_framework::aggregator_v2 {

    spec Aggregator {
        pragma intrinsic;
    }

    spec max_value<IntElement: copy + drop>(self: &Aggregator<IntElement>): IntElement {
        pragma intrinsic;
    }

    spec create_aggregator<IntElement: copy + drop>(max_value: IntElement): Aggregator<IntElement> {
        pragma intrinsic;
    }

    spec create_unbounded_aggregator<IntElement: copy + drop>(): Aggregator<IntElement> {
        pragma intrinsic;
    }

    spec try_add<IntElement>(self: &mut Aggregator<IntElement>, value: IntElement): bool {
        pragma intrinsic;
    }

    spec add<IntElement>(self: &mut Aggregator<IntElement>, value: IntElement) {
        pragma intrinsic;
    }

    spec try_sub<IntElement>(self: &mut Aggregator<IntElement>, value: IntElement): bool {
        pragma intrinsic;
    }

    spec sub<IntElement>(self: &mut Aggregator<IntElement>, value: IntElement) {
        pragma intrinsic;
    }

    spec is_at_least_impl<IntElement>(self: &Aggregator<IntElement>, min_amount: IntElement): bool {
        pragma intrinsic;
    }

    spec read<IntElement>(self: &Aggregator<IntElement>): IntElement {
        pragma intrinsic;
    }

    spec snapshot<IntElement>(self: &Aggregator<IntElement>): AggregatorSnapshot<IntElement> {
        pragma opaque;
        include AbortsIfIntElement<IntElement>;
        ensures [abstract] result.value == spec_get_value(self);
    }

    spec create_snapshot<IntElement: copy + drop>(value: IntElement): AggregatorSnapshot<IntElement> {
        pragma opaque;
        include AbortsIfIntElement<IntElement>;
        ensures [abstract] result.value == value;
    }

    spec read_snapshot<IntElement>(self: &AggregatorSnapshot<IntElement>): IntElement {
        pragma opaque;
        include AbortsIfIntElement<IntElement>;
        ensures [abstract] result == self.value;
    }

    spec read_derived_string(self: &DerivedStringSnapshot): String {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == self.value;
    }

    spec create_derived_string(value: String): DerivedStringSnapshot {
        pragma opaque;
        aborts_if [abstract] len(value.bytes) > 1024;
        ensures [abstract] result.value == value;
    }

    spec derive_string_concat<IntElement>(before: String, snapshot: &AggregatorSnapshot<IntElement>, after: String): DerivedStringSnapshot {
        pragma opaque;
        include AbortsIfIntElement<IntElement>;
        ensures [abstract] result.value.bytes == concat(before.bytes, concat(spec_get_string_value(snapshot).bytes, after.bytes));
        aborts_if [abstract] len(before.bytes) + len(after.bytes) > 1024;
    }

    spec schema AbortsIfIntElement<IntElement> {
        use aptos_std::type_info;
        aborts_if [abstract] type_info::type_name<IntElement>().bytes != b"u64" && type_info::type_name<IntElement>().bytes != b"u128";
    }

    // deprecated
    spec copy_snapshot {
        pragma opaque;
        aborts_if [abstract] true;
    }

    // deprecated
    spec string_concat {
        pragma opaque;
        aborts_if [abstract] true;
    }

    // Get aggregator.value
    spec native fun spec_get_value<IntElement>(aggregator: Aggregator<IntElement>): IntElement;
    // Get aggregator.max_value
    spec native fun spec_get_max_value<IntElement>(aggregator: Aggregator<IntElement>): IntElement;
    // Uninterpreted spec function that translates the value inside aggregator into corresponding string representation
    spec fun spec_get_string_value<IntElement>(aggregator: AggregatorSnapshot<IntElement>): String;
    spec fun spec_read_snapshot<IntElement>(snapshot: AggregatorSnapshot<IntElement>): IntElement {
        snapshot.value
    }
    spec fun spec_read_derived_string(snapshot: DerivedStringSnapshot): String {
        snapshot.value
    }
}
