module 0x1::aggregator_v2_test {
    use velor_framework::aggregator_v2::{Self, Aggregator, AggregatorSnapshot, DerivedStringSnapshot};
    use velor_std::debug;
    use velor_std::table::{Self, Table};
    use std::vector;
    use std::string::String;
    use std::option::{Self, Option};

    const USE_RESOURCE_TYPE: u32 = 0;
    const USE_TABLE_TYPE: u32 = 1;
    const USE_RESOURCE_GROUP_TYPE: u32 = 2;

    /// When checking the value of aggregator fails.
    const ENOT_EQUAL: u64 = 17;

    const EINVALID_ARG: u64 = 18;

    const ERESOURCE_DOESNT_EXIST: u64 = 19;
    const ETABLE_DOESNT_EXIST: u64 = 20;
    const ERESOURCE_GROUP_DOESNT_EXIST: u64 = 21;
    const EINDEX_DOESNT_EXIST: u64 = 22;
    const EOPTION_DOESNT_EXIST: u64 = 23;

    const ERESOURCE_ALREADY_EXISTS: u64 = 24;
    const ETABLE_ALREADY_EXISTS: u64 = 25;
    const ERESOURCE_GROUP_ALREADY_EXISTS: u64 = 26;

    struct AggregatorInResource<Agg: store + drop> has key, store, drop {
        data: vector<Option<Agg>>,
    }

    /// Resource to store aggregators/snapshots. Each aggregator is associated with a
    /// determinictic integer value, for testing purposes.
    /// We put multiple (10) aggregators/snapshots into same resource, to be
    /// able to test multiple aggregators/snapshots being inside same resource.
    struct AggregatorInTable<Agg: store + drop> has key, store {
        data: Table<u64, vector<Option<Agg>>>,
    }

    #[resource_group(scope = global)]
    struct MyGroup {}

    #[resource_group_member(group = 0x1::aggregator_v2_test::MyGroup)]
    struct AggregatorInResourceGroup<Agg: store + drop> has key, drop {
        data: vector<Option<Agg>>,
    }

    public entry fun verify_string_concat() {
        let snapshot = aggregator_v2::create_snapshot(42);
        let snapshot2 = aggregator_v2::derive_string_concat(std::string::utf8(b"before"), &snapshot, std::string::utf8(b"after"));
        let val = aggregator_v2::read_derived_string(&snapshot2);

        debug::print(&val);
        debug::print(&std::string::utf8(b"before42after"));
        assert!(val == std::string::utf8(b"before42after"), 5);
    }

    fun init<Agg: store + drop>(account: &signer, use_type: u32) {
        let addr = std::signer::address_of(account);
        if (use_type == USE_RESOURCE_TYPE) {
            assert!(!exists<AggregatorInResource<Agg>>(addr), ERESOURCE_ALREADY_EXISTS);
            move_to(account, AggregatorInResource<Agg> { data: vector::empty() });
        } else if (use_type == USE_TABLE_TYPE) {
            assert!(!exists<AggregatorInTable<Agg>>(addr), ETABLE_ALREADY_EXISTS);
            move_to(account, AggregatorInTable<Agg> { data: table::new() });
        } else if (use_type == USE_RESOURCE_GROUP_TYPE) {
            assert!(!exists<AggregatorInTable<Agg>>(addr), ERESOURCE_GROUP_ALREADY_EXISTS);
            move_to(account, AggregatorInResourceGroup<Agg> { data: vector::empty() });
        } else {
            assert!(false, EINVALID_ARG);
        };
    }

    public entry fun init_aggregator<Element: store + drop>(account: &signer, use_type: u32) {
        init<Aggregator<Element>>(account, use_type);
    }

    public entry fun init_snapshot<Element: store + drop>(account: &signer, use_type: u32) {
        init<AggregatorSnapshot<Element>>(account, use_type);
    }

    public entry fun init_derived_string<Element: store>(account: &signer, use_type: u32) {
        init<DerivedStringSnapshot>(account, use_type);
    }

    fun delete<Agg: store + drop>(account_addr: address, use_type: u32) acquires AggregatorInResource, AggregatorInResourceGroup {
        if (use_type == USE_RESOURCE_TYPE) {
            assert!(exists<AggregatorInResource<Agg>>(account_addr), ERESOURCE_DOESNT_EXIST);
            move_from<AggregatorInResource<Agg>>(account_addr);
        // } else if (use_type == USE_TABLE_TYPE) {
        //     move_from<AggregatorInTable<Agg>>(account_addr);
        } else if (use_type == USE_RESOURCE_GROUP_TYPE) {
            assert!(exists<AggregatorInResourceGroup<Agg>>(account_addr), ERESOURCE_GROUP_DOESNT_EXIST);
            move_from<AggregatorInResourceGroup<Agg>>(account_addr);
        } else {
            assert!(false, EINVALID_ARG);
        };
    }

    public entry fun delete_aggregator<Element: store + drop>(account_addr: address, use_type: u32) acquires AggregatorInResource, AggregatorInResourceGroup {
        delete<Aggregator<Element>>(account_addr, use_type);
    }

    public entry fun delete_snapshot<Element: store + drop>(account_addr: address, use_type: u32) acquires AggregatorInResource, AggregatorInResourceGroup {
        delete<AggregatorSnapshot<Element>>(account_addr, use_type);
    }

    public entry fun delete_derived_string<Element: store + drop>(account_addr: address, use_type: u32) acquires AggregatorInResource, AggregatorInResourceGroup {
        delete<DerivedStringSnapshot>(account_addr, use_type);
    }

    fun insert<Agg: store + drop>(account_addr: address, use_type: u32, i: u64, e: Agg) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        assert!(use_type == USE_RESOURCE_TYPE || use_type == USE_TABLE_TYPE || use_type == USE_RESOURCE_GROUP_TYPE, EINVALID_ARG);

        let vector_data = if (use_type == USE_RESOURCE_TYPE) {
            assert!(exists<AggregatorInResource<Agg>>(account_addr), ERESOURCE_DOESNT_EXIST);
            &mut borrow_global_mut<AggregatorInResource<Agg>>(account_addr).data
        } else if (use_type == USE_TABLE_TYPE) {
            assert!(exists<AggregatorInTable<Agg>>(account_addr), ETABLE_DOESNT_EXIST);
            let data = &mut borrow_global_mut<AggregatorInTable<Agg>>(account_addr).data;
            let outer = i / 10;
            let inner = i % 10;
            i = inner;
            if (!table::contains(data, outer)) {
                table::add(data, outer, vector::empty());
            };

            table::borrow_mut(data, outer)
        } else { // if (use_type == USE_RESOURCE_GROUP_TYPE) {
            assert!(exists<AggregatorInResourceGroup<Agg>>(account_addr), ERESOURCE_GROUP_DOESNT_EXIST);
            &mut borrow_global_mut<AggregatorInResourceGroup<Agg>>(account_addr).data
        };

        if (vector::length(vector_data) == i) {
            vector::push_back(vector_data, option::some(e));
        } else {
            assert!(vector::length(vector_data) > i, EINDEX_DOESNT_EXIST);
            let option_data = vector::borrow_mut(vector_data, i);
            option::swap_or_fill(option_data, e);
        };
    }

    inline fun for_element_ref<Agg: store + drop, R>(account_addr: address, use_type: u32, i: u64, f: |&Agg|R): R acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        assert!(use_type == USE_RESOURCE_TYPE || use_type == USE_TABLE_TYPE || use_type == USE_RESOURCE_GROUP_TYPE, EINVALID_ARG);
        let vector_data = if (use_type == USE_RESOURCE_TYPE) {
            assert!(exists<AggregatorInResource<Agg>>(account_addr), ERESOURCE_DOESNT_EXIST);
            &borrow_global<AggregatorInResource<Agg>>(account_addr).data
        } else if (use_type == USE_TABLE_TYPE) {
            assert!(exists<AggregatorInTable<Agg>>(account_addr), ETABLE_DOESNT_EXIST);
            let data = &borrow_global<AggregatorInTable<Agg>>(account_addr).data;
            let outer = i / 10;
            let inner = i % 10;
            i = inner;
            table::borrow(data, outer)
        } else { // if (use_type == USE_RESOURCE_GROUP_TYPE) {
            assert!(exists<AggregatorInResourceGroup<Agg>>(account_addr), ERESOURCE_GROUP_DOESNT_EXIST);
            &borrow_global<AggregatorInResourceGroup<Agg>>(account_addr).data
        };

        assert!(vector::length(vector_data) > i, EINDEX_DOESNT_EXIST);
        let option_data = vector::borrow(vector_data, i);
        assert!(option::is_some(option_data), EOPTION_DOESNT_EXIST);
        let value = option::borrow(option_data);

        f(value)
    }

    inline fun for_element_mut<Agg: store + drop, R>(account_addr: address, use_type: u32, i: u64, f: |&mut Agg|R): R acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        assert!(use_type == USE_RESOURCE_TYPE || use_type == USE_TABLE_TYPE || use_type == USE_RESOURCE_GROUP_TYPE, EINVALID_ARG);
        let vector_data = if (use_type == USE_RESOURCE_TYPE) {
            &mut borrow_global_mut<AggregatorInResource<Agg>>(account_addr).data
        } else if (use_type == USE_TABLE_TYPE) {
            let data = &mut borrow_global_mut<AggregatorInTable<Agg>>(account_addr).data;
            let outer = i / 10;
            let inner = i % 10;
            i = inner;
            table::borrow_mut(data, outer)
        } else { // if (use_type == USE_RESOURCE_GROUP_TYPE) {
            &mut borrow_global_mut<AggregatorInResourceGroup<Agg>>(account_addr).data
        };

        let option_data = vector::borrow_mut(vector_data, i);
        let value = option::borrow_mut(option_data);
        f(value)
    }

    public entry fun new<Element: drop + copy + store>(addr: address, use_type: u32, i: u64, limit: Element) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        insert<Aggregator<Element>>(addr, use_type, i, aggregator_v2::create_aggregator(limit));
    }

    public fun call_read<Element: store + drop>(addr: address, use_type: u32, i: u64): Element acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        for_element_ref<Aggregator<Element>, Element>(addr, use_type, i, |aggregator| aggregator_v2::read(aggregator))
    }

    public entry fun try_add<Element: store + drop>(addr: address, use_type: u32, i: u64, value: Element) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        for_element_mut<Aggregator<Element>, bool>(addr, use_type, i, |aggregator| aggregator_v2::try_add(aggregator, value));
    }

    public entry fun add<Element: store + drop>(addr: address, use_type: u32, i: u64, value: Element) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        for_element_mut<Aggregator<Element>, bool>(addr, use_type, i, |aggregator| {
            aggregator_v2::add(aggregator, value);
            true
        });
    }

    public entry fun try_sub<Element: store + drop>(addr: address, use_type: u32, i: u64, value: Element) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        for_element_mut<Aggregator<Element>, bool>(addr, use_type, i, |aggregator| aggregator_v2::try_sub(aggregator, value));
    }

    public entry fun sub<Element: store + drop>(addr: address, use_type: u32, i: u64, value: Element) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        for_element_mut<Aggregator<Element>, bool>(addr, use_type, i, |aggregator| {
            aggregator_v2::sub(aggregator, value);
            true
        });
    }

    public entry fun materialize<Element: store + drop>(addr: address, use_type: u32, i: u64) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        call_read<Element>(addr, use_type, i);
    }

    /// Checks that the ith aggregator has expected value. Useful to inject into
    /// transaction block to verify successful and correct execution.
    public entry fun check<Element: store + drop>(addr: address, use_type: u32, i: u64, expected: Element) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        let actual = call_read<Element>(addr, use_type, i);
        assert!(actual == expected, ENOT_EQUAL)
    }

    public entry fun new_add<Element: drop + copy + store>(addr: address, use_type: u32, i: u64, limit: Element, a: Element) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        new(addr, use_type, i, limit);
        add(addr, use_type, i, a);
    }

    public entry fun sub_add<Element: store + drop>(addr: address, use_type: u32, i: u64, a: Element, b: Element) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        for_element_mut<Aggregator<Element>, bool>(addr, use_type, i, |aggregator| {
            aggregator_v2::sub(aggregator, a);
            aggregator_v2::add(aggregator, b);
            true
        });
    }

    public entry fun add_if_at_least<Element: store + drop>(addr: address, use_type: u32, i: u64, a: Element, b: Element) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        let is_at_least = for_element_ref<Aggregator<Element>, bool>(addr, use_type, i, |aggregator| {
            aggregator_v2::is_at_least(aggregator, a)
        });

        if (is_at_least) {
            add<Element>(addr, use_type, i, b);
        }

        // issue with type inference of lambda or downcasting from mut to non-mut?
        // for_element_mut<Aggregator<Element>, bool>(addr, use_type, i, |aggregator| {
        //     if (aggregator_v2::is_at_least(aggregator, a)) {
        //         aggregator_v2::add(aggregator, b);
        //     };
        //     true
        // });
    }

    public entry fun add_sub<Element: store + drop>(addr: address, use_type: u32, i: u64, a: Element, b: Element) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        for_element_mut<Aggregator<Element>, bool>(addr, use_type, i, |aggregator| {
            aggregator_v2::add(aggregator, b);
            aggregator_v2::sub(aggregator, a);
            true
        });
    }

    public entry fun add_delete<Element: drop + copy + store>(addr: address, use_type: u32, i: u64, a: Element) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        add(addr, use_type, i, a);
        delete_aggregator<Element>(addr, use_type);
    }

    public entry fun materialize_and_add<Element: store + drop>(addr: address, use_type: u32, i: u64, value: Element) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        call_read<Element>(addr, use_type, i);
        add<Element>(addr, use_type, i, value);

        // issue with type inference of lambda?
        // for_element_mut<Aggregator<u128>, bool>(account, use_type, i, |aggregator| {
        //     aggregator_v2::read(aggregator);
        //     aggregator_v2::try_add(aggregator, value)
        // });
    }

    public entry fun materialize_and_sub<Element: store + drop>(addr: address, use_type: u32, i: u64, value: Element) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        call_read<Element>(addr, use_type, i);
        sub<Element>(addr, use_type, i, value);

        // issue with type inference of lambda?
        // for_element_mut<Aggregator<u128>, bool>(account, use_type, i, |aggregator| {
        //     aggregator_v2::read(aggregator);
        //     aggregator_v2::try_sub(aggregator, value)
        // });
    }

    public entry fun add_and_materialize<Element: store + drop>(addr: address, use_type: u32, i: u64, value: Element) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        for_element_mut<Aggregator<Element>, Element>(addr, use_type, i, |aggregator| {
            aggregator_v2::add(aggregator, value);
            aggregator_v2::read(aggregator)
        });
    }

    public entry fun sub_and_materialize<Element: store + drop>(addr: address, use_type: u32, i: u64, value: Element) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        for_element_mut<Aggregator<Element>, Element>(addr, use_type, i, |aggregator| {
            aggregator_v2::sub(aggregator, value);
            aggregator_v2::read(aggregator)
        });
    }

    public entry fun add_2<A: store + drop, B: store + drop>(addr_a: address, use_type_a: u32, i_a: u64, a: A, addr_b: address, use_type_b: u32, i_b: u64, b: B) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        add<A>(addr_a, use_type_a, i_a, a);
        add<B>(addr_b, use_type_b, i_b, b);
    }

    public entry fun snapshot<Element: store + drop>(addr_i: address, use_type_i: u32, i: u64, addr_j: address, use_type_j: u32, j: u64) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        let snapshot = for_element_ref<Aggregator<Element>, AggregatorSnapshot<Element>>(addr_i, use_type_i, i, |aggregator| {
            aggregator_v2::snapshot<Element>(aggregator)
        });
        insert<AggregatorSnapshot<Element>>(addr_j, use_type_j, j, snapshot);
    }

    public entry fun concat<Element: store + drop>(addr_i: address, use_type_i: u32, i: u64, addr_j: address, use_type_j: u32, j: u64, prefix: String, suffix: String) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        let snapshot = for_element_ref<AggregatorSnapshot<Element>, DerivedStringSnapshot>(addr_i, use_type_i, i, |snapshot| {
            aggregator_v2::derive_string_concat<Element>(prefix, snapshot, suffix)
        });
        insert<DerivedStringSnapshot>(addr_j, use_type_j, j, snapshot);
    }

    public entry fun read_snapshot<Element: store + drop>(addr: address, use_type: u32, i: u64) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        for_element_ref<AggregatorSnapshot<Element>, Element>(addr, use_type, i, |snapshot| aggregator_v2::read_snapshot(snapshot));
    }

    public entry fun check_snapshot<Element: store + drop>(addr: address, use_type: u32, i: u64, expected: Element) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        let actual = for_element_ref<AggregatorSnapshot<Element>, Element>(addr, use_type, i, |snapshot| aggregator_v2::read_snapshot(snapshot));
        assert!(actual == expected, ENOT_EQUAL)
    }

    public entry fun check_derived<Element: store + drop>(addr: address, use_type: u32, i: u64, expected: String) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        let actual = for_element_ref<DerivedStringSnapshot, String>(addr, use_type, i, |snapshot| aggregator_v2::read_derived_string(snapshot));
        assert!(actual == expected, ENOT_EQUAL)
    }

    public entry fun add_and_read_snapshot<Element: copy + store + drop>(addr: address, use_type: u32, i: u64, value: Element) acquires AggregatorInResource, AggregatorInTable, AggregatorInResourceGroup {
        for_element_mut<Aggregator<Element>, Element>(addr, use_type, i, |aggregator| {
            aggregator_v2::add(aggregator, value);
            aggregator_v2::sub(aggregator, value);
            let aggregator_snapshot_1 = aggregator_v2::snapshot(aggregator);
            aggregator_v2::add(aggregator, value);
            let aggregator_snapshot_2 = aggregator_v2::snapshot(aggregator);
            aggregator_v2::add(aggregator, value);
            let aggregator_snapshot_3 = aggregator_v2::snapshot(aggregator);
            let _snapshot_value_1 = aggregator_v2::read_snapshot(&aggregator_snapshot_1);
            let _snapshot_value_2 = aggregator_v2::read_snapshot(&aggregator_snapshot_2);
            let snapshot_value_3 = aggregator_v2::read_snapshot(&aggregator_snapshot_3);
            // assert!(snapshot_value_2 == snapshot_value_1 + value, ENOT_EQUAL);
            // assert!(snapshot_value_3 == snapshot_value_2 + value, ENOT_EQUAL);
            snapshot_value_3
        });
    }

    #[test]
    fun test_verify_string_concat() {
        verify_string_concat();
    }
}
