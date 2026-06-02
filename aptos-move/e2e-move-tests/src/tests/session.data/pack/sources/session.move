module 0x1::session {
    use std::signer;
    use std::vector;
    use aptos_framework::aggregator_v2::{Aggregator, Self};

    fun init_module(account: &signer) {
        let test_1 = Test1 {
            data: vector::empty<u64>(),
            aggregator: aggregator_v2::create_unbounded_aggregator<u64>(),
        };
        move_to(account, test_1);

        let test_2 = Test2 {
            aggregator: aggregator_v2::create_unbounded_aggregator<u64>(),
        };
        move_to(account, test_2);

        let b1 = BGroup1 {
            data: vector::singleton(123),
            aggregator: aggregator_v2::create_unbounded_aggregator<u64>(),
        };
        move_to(account, b1);
    }

    // Testcases:

    struct Test1 has key, drop {
        data: vector<u64>,
        aggregator: Aggregator<u64>,
    }

    struct Test2 has key, drop {
        aggregator: Aggregator<u64>,
    }

    struct Tmp has key, drop {
        data: u128,
        aggregator: Aggregator<u64>,
    }

    #[resource_group(scope = global)]
    struct Group1 {}

    #[resource_group_member(group = 0x1::session::Group1)]
    struct AGroup1 has key, drop {}

    #[resource_group_member(group = 0x1::session::Group1)]
    struct BGroup1 has key, drop {
        data: vector<u128>,
        aggregator: Aggregator<u64>,
    }

    #[resource_group(scope = global)]
    struct Group2 {}

    #[resource_group_member(group = 0x1::session::Group2)]
    struct AGroup2 has key, drop {
        data: vector<u128>,
        aggregator: Aggregator<u64>,
    }

    #[resource_group_member(group = 0x1::session::Group2)]
    struct BGroup2 has key, drop {
        data: vector<u128>,
    }

    public entry fun test_1_change_resource_size(account: &signer) acquires Test1 {
        let addr = signer::address_of(account);
        let resource = borrow_global_mut<Test1>(addr);
        vector::push_back(&mut resource.data, 1);
        vector::push_back(&mut resource.data, 2);
    }

    public entry fun test_1_increment_aggregator(account: &signer) acquires Test1 {
        let addr = signer::address_of(account);
        let resource = borrow_global_mut<Test1>(addr);
        aggregator_v2::add(&mut resource.aggregator, 1);
    }

    public entry fun test_2_move_aggregator(account: &signer) acquires Test2 {
        let addr = signer::address_of(account);
        let Test2 { aggregator } = move_from<Test2>(addr);
        let tmp = Tmp {
            data: 12345,
            aggregator,
        };
        move_to(account, tmp);
    }

    public entry fun test_2_increment_aggregator(account: &signer) acquires Tmp {
        let addr = signer::address_of(account);
        let resource = borrow_global_mut<Tmp>(addr);
        aggregator_v2::add(&mut resource.aggregator, 1);
    }

    public entry fun test_3_increment_fst_aggregator(account: &signer) acquires Test1 {
        let addr = signer::address_of(account);
        let resource = borrow_global_mut<Test1>(addr);
        aggregator_v2::add(&mut resource.aggregator, 100);
    }

    public entry fun test_3_increment_snd_aggregator(account: &signer) acquires Test2 {
        let addr = signer::address_of(account);
        let resource = borrow_global_mut<Test2>(addr);
        aggregator_v2::add(&mut resource.aggregator, 200);
    }

    public entry fun test_4_write_fst_resource(account: &signer) acquires Test1 {
        let addr = signer::address_of(account);
        let resource = borrow_global_mut<Test1>(addr);
        vector::push_back(&mut resource.data, 1);
        vector::push_back(&mut resource.data, 2);
        vector::push_back(&mut resource.data, 3);
    }

    public entry fun test_4_write_snd_resource(account: &signer) acquires Test2 {
        let addr = signer::address_of(account);
        let resource = borrow_global_mut<Test2>(addr);
        *resource = Test2 {
            aggregator: aggregator_v2::create_unbounded_aggregator<u64>(),
        };
    }

    public entry fun test_5_change_resource_group_size(account: &signer){
        move_to(account, AGroup1 {})
    }

    public entry fun test_5_increment_aggregator(account: &signer) acquires BGroup1 {
        let addr = signer::address_of(account);
        let resource = borrow_global_mut<BGroup1>(addr);
        aggregator_v2::add(&mut resource.aggregator, 1);
    }

    public entry fun test_6_move_aggregator_between_groups(account: &signer) acquires BGroup1 {
        let addr = signer::address_of(account);
        let BGroup1 { data: _, aggregator } = move_from<BGroup1>(addr);
        let a2 = AGroup2 {
            data: vector::singleton(1),
            aggregator,
        };
        move_to(account, a2);
    }

    public entry fun test_6_increment_aggregator(account: &signer) acquires AGroup2 {
        let addr = signer::address_of(account);
        let resource = borrow_global_mut<AGroup2>(addr);
        aggregator_v2::add(&mut resource.aggregator, 1);
    }

    public entry fun test_7_move_aggregator_from_group_to_resource(account: &signer) acquires BGroup1 {
        let addr = signer::address_of(account);
        let BGroup1 { data: _, aggregator } = move_from<BGroup1>(addr);
        let tmp = Tmp {
            data: 0,
            aggregator,
        };
        move_to(account, tmp);
    }

    public entry fun test_7_increment_aggregator(account: &signer) acquires Tmp {
        let addr = signer::address_of(account);
        let resource = borrow_global_mut<Tmp>(addr);
        aggregator_v2::add(&mut resource.aggregator, 1);
    }

    /// Full WRITE of a group member's non-delayed field (BGroup1.data) WITHOUT touching the
    /// aggregator. Produces a `WriteResourceGroup` on Group1 with the aggregator left as a
    /// placeholder, so it can be paired to construct `WriteResourceGroup`-vs-* arms while the
    /// aggregator value flows through the delayed-field change set.
    public entry fun test_8_write_group_member_data(account: &signer) acquires BGroup1 {
        let addr = signer::address_of(account);
        let resource = borrow_global_mut<BGroup1>(addr);
        vector::push_back(&mut resource.data, 999);
    }
}
