/// Defines the charges associated with using a marketplace, namely:
/// * Listing rate, the units charged for creating a listing.
/// * Bidding rate, the units per bid made by a potential buyer.
/// * Commission, the units transferred to the marketplace upon sale.
module marketplace::fee_schedule {
    use std::error;
    use std::signer;
    use std::string::{Self, String};
    use velor_std::math64;

    use velor_std::type_info;

    use velor_framework::event;
    use velor_framework::object::{Self, ConstructorRef, ExtendRef, Object};

    /// FeeSchedule does not exist.
    const ENO_FEE_SCHEDULE: u64 = 1;
    /// The denominator in a fraction cannot be zero.
    const EDENOMINATOR_IS_ZERO: u64 = 2;
    /// The value represented by a fraction cannot be greater than 1.
    const EEXCEEDS_MAXIMUM: u64 = 3;
    /// The passed in signer is not the owner of the marketplace.
    const ENOT_OWNER: u64 = 4;

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// Defines marketplace fees
    struct FeeSchedule has key {
        /// Address to send fees to
        fee_address: address,
        /// Ref for changing the configuration of the marketplace
        extend_ref: ExtendRef,
    }

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// Fixed rate for bidding
    struct FixedRateBiddingFee has drop, key {
        /// Fixed rate for bidding
        bidding_fee: u64,
    }

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// Fixed rate for listing
    struct FixedRateListingFee has drop, key {
        /// Fixed rate for listing
        listing_fee: u64,
    }

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// Fixed rate for commission
    struct FixedRateCommission has drop, key {
        /// Fixed rate for commission
        commission: u64,
    }

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// Percentage-based rate for commission
    struct PercentageRateCommission has drop, key {
        /// Denominator for the commission rate
        denominator: u64,
        /// Numerator for the commission rate
        numerator: u64,
    }

    #[event]
    /// Event representing a change to the marketplace configuration
    struct Mutation has drop, store {
        marketplace: address,
        /// The type info of the struct that was updated.
        updated_resource: String,
    }

    // Initializers

    /// Create a marketplace with a fixed bidding and listing rate and a percentage commission.
    public entry fun init_entry(
        creator: &signer,
        fee_address: address,
        bidding_fee: u64,
        listing_fee: u64,
        commission_denominator: u64,
        commission_numerator: u64,
    ) {
        init(
            creator,
            fee_address,
            bidding_fee,
            listing_fee,
            commission_denominator,
            commission_numerator,
        );
    }


    public fun init(
        creator: &signer,
        fee_address: address,
        bidding_fee: u64,
        listing_fee: u64,
        commission_denominator: u64,
        commission_numerator: u64,
    ): Object<FeeSchedule> {
        assert!(
            commission_numerator <= commission_denominator,
            error::invalid_argument(EEXCEEDS_MAXIMUM),
        );
        assert!(
            commission_denominator != 0,
            error::out_of_range(EDENOMINATOR_IS_ZERO),
        );

        let (constructor_ref, fee_schedule_signer) = empty_init(creator, fee_address);
        move_to(&fee_schedule_signer, FixedRateBiddingFee { bidding_fee });
        move_to(&fee_schedule_signer, FixedRateListingFee { listing_fee });
        let commission_rate = PercentageRateCommission {
            denominator: commission_denominator,
            numerator: commission_numerator,
        };
        move_to(&fee_schedule_signer, commission_rate);
        object::object_from_constructor_ref(&constructor_ref)
    }

    /// Create a marketplace with no fees.
    public entry fun empty(creator: &signer, fee_address: address) {
        empty_init(creator, fee_address);
    }

    inline fun empty_init(creator: &signer, fee_address: address): (ConstructorRef, signer) {
        let constructor_ref = object::create_object_from_account(creator);
        let extend_ref = object::generate_extend_ref(&constructor_ref);
        let fee_schedule_signer = object::generate_signer(&constructor_ref);

        let marketplace = FeeSchedule {
            fee_address,
            extend_ref,
        };
        move_to(&fee_schedule_signer, marketplace);

        (constructor_ref, fee_schedule_signer)
    }

    // Mutators

    /// Set the fee address
    public entry fun set_fee_address(
        creator: &signer,
        marketplace: Object<FeeSchedule>,
        fee_address: address,
    ) acquires FeeSchedule {
        let fee_schedule_addr = assert_exists_internal(&marketplace);
        assert!(
            object::is_owner(marketplace, signer::address_of(creator)),
            error::permission_denied(ENOT_OWNER),
        );
        let fee_schedule_obj = borrow_global_mut<FeeSchedule>(fee_schedule_addr);
        fee_schedule_obj.fee_address = fee_address;
        let updated_resource = string::utf8(b"fee_address");
        event::emit(Mutation { marketplace: fee_schedule_addr, updated_resource });
    }

    /// Remove any existing listing fees and set a fixed rate listing fee.
    public entry fun set_fixed_rate_listing_fee(
        creator: &signer,
        marketplace: Object<FeeSchedule>,
        fee: u64,
    ) acquires FeeSchedule, FixedRateListingFee {
        let fee_schedule_signer = remove_listing_fee(creator, marketplace);
        move_to(&fee_schedule_signer, FixedRateListingFee { listing_fee: fee });
        let updated_resource = type_info::type_name<FixedRateListingFee>();
        event::emit(Mutation { marketplace: signer::address_of(&fee_schedule_signer), updated_resource });
    }

    inline fun remove_listing_fee(
        creator: &signer,
        marketplace: Object<FeeSchedule>,
    ): signer acquires FeeSchedule, FixedRateListingFee {
        let (fee_schedule_signer, fee_schedule_addr) = assert_access(creator, marketplace);
        if (exists<FixedRateListingFee>(fee_schedule_addr)) {
            move_from<FixedRateListingFee>(fee_schedule_addr);
        };
        fee_schedule_signer
    }

    /// Remove any existing bidding fees and set a fixed rate bidding fee.
    public entry fun set_fixed_rate_bidding_fee(
        creator: &signer,
        marketplace: Object<FeeSchedule>,
        fee: u64,
    ) acquires FeeSchedule, FixedRateBiddingFee {
        let fee_schedule_signer = remove_bidding_fee(creator, marketplace);
        move_to(&fee_schedule_signer, FixedRateBiddingFee { bidding_fee: fee });
        let updated_resource = type_info::type_name<FixedRateListingFee>();
        event::emit(Mutation { marketplace: signer::address_of(&fee_schedule_signer), updated_resource });
    }

    inline fun remove_bidding_fee(
        creator: &signer,
        marketplace: Object<FeeSchedule>,
    ): signer acquires FeeSchedule, FixedRateBiddingFee {
        let (fee_schedule_signer, fee_schedule_addr) = assert_access(creator, marketplace);
        if (exists<FixedRateBiddingFee>(fee_schedule_addr)) {
            move_from<FixedRateBiddingFee>(fee_schedule_addr);
        };
        fee_schedule_signer
    }

    /// Remove any existing commission and set a fixed rate commission.
    public entry fun set_fixed_rate_commission(
        creator: &signer,
        marketplace: Object<FeeSchedule>,
        commission: u64,
    ) acquires FeeSchedule, FixedRateCommission, PercentageRateCommission {
        let fee_schedule_signer = remove_commission(creator, marketplace);
        move_to(&fee_schedule_signer, FixedRateCommission { commission });
        let updated_resource = type_info::type_name<FixedRateListingFee>();
        event::emit(Mutation { marketplace: signer::address_of(&fee_schedule_signer), updated_resource });
    }

    /// Remove any existing commission and set a percentage rate commission.
    public entry fun set_percentage_rate_commission(
        creator: &signer,
        marketplace: Object<FeeSchedule>,
        denominator: u64,
        numerator: u64,
    ) acquires FeeSchedule, FixedRateCommission, PercentageRateCommission {
        assert!(
            numerator <= denominator,
            error::invalid_argument(EEXCEEDS_MAXIMUM),
        );
        assert!(
            denominator != 0,
            error::out_of_range(EDENOMINATOR_IS_ZERO),
        );

        let fee_schedule_signer = remove_commission(creator, marketplace);
        move_to(&fee_schedule_signer, PercentageRateCommission { denominator, numerator });
        let updated_resource = type_info::type_name<FixedRateListingFee>();
        event::emit(Mutation { marketplace: signer::address_of(&fee_schedule_signer), updated_resource });
    }

    inline fun remove_commission(
        creator: &signer,
        marketplace: Object<FeeSchedule>,
    ): signer acquires FeeSchedule, FixedRateCommission, PercentageRateCommission {
        let (fee_schedule_signer, fee_schedule_addr) = assert_access(creator, marketplace);
        if (exists<FixedRateCommission>(fee_schedule_addr)) {
            move_from<FixedRateCommission>(fee_schedule_addr);
        } else if (exists<PercentageRateCommission>(fee_schedule_addr)) {
            move_from<PercentageRateCommission>(fee_schedule_addr);
        };
        fee_schedule_signer
    }

    inline fun assert_access(
        creator: &signer,
        marketplace: Object<FeeSchedule>,
    ): (signer, address) acquires FeeSchedule {
        let fee_schedule_addr = assert_exists_internal(&marketplace);
        assert!(
            object::is_owner(marketplace, signer::address_of(creator)),
            error::permission_denied(ENOT_OWNER),
        );
        let fee_schedule_obj = borrow_global<FeeSchedule>(fee_schedule_addr);
        let fee_schedule_signer = object::generate_signer_for_extending(&fee_schedule_obj.extend_ref);
        (fee_schedule_signer, fee_schedule_addr)
    }

    // View functions
    #[view]
    public fun fee_address(marketplace: Object<FeeSchedule>): address acquires FeeSchedule {
        let fee_schedule_addr = assert_exists_internal(&marketplace);
        borrow_global<FeeSchedule>(fee_schedule_addr).fee_address
    }

    #[view]
    public fun listing_fee(
        marketplace: Object<FeeSchedule>,
        _base: u64,
    ): u64 acquires FixedRateListingFee {
        let fee_schedule_addr = assert_exists_internal(&marketplace);
        if (exists<FixedRateListingFee>(fee_schedule_addr)) {
            borrow_global<FixedRateListingFee>(fee_schedule_addr).listing_fee
        } else {
            0
        }
    }

    #[view]
    public fun bidding_fee(
        marketplace: Object<FeeSchedule>,
        _bid: u64,
    ): u64 acquires FixedRateBiddingFee {
        let fee_schedule_addr = assert_exists_internal(&marketplace);
        if (exists<FixedRateBiddingFee>(fee_schedule_addr)) {
            borrow_global<FixedRateBiddingFee>(fee_schedule_addr).bidding_fee
        } else {
            0
        }
    }

    #[view]
    public fun commission(
        marketplace: Object<FeeSchedule>,
        price: u64,
    ): u64 acquires FixedRateCommission, PercentageRateCommission {
        let fee_schedule_addr = assert_exists_internal(&marketplace);
        if (exists<FixedRateCommission>(fee_schedule_addr)) {
            borrow_global<FixedRateCommission>(fee_schedule_addr).commission
        } else if (exists<PercentageRateCommission>(fee_schedule_addr)) {
            let fees = borrow_global<PercentageRateCommission>(fee_schedule_addr);
            math64::mul_div(price, fees.numerator, fees.denominator)
        } else {
            0
        }
    }

    public fun assert_exists(marketplace: &Object<FeeSchedule>) {
        assert_exists_internal(marketplace);
    }

    inline fun assert_exists_internal(marketplace: &Object<FeeSchedule>): address {
        let fee_schedule_addr = object::object_address(marketplace);
        assert!(
            exists<FeeSchedule>(fee_schedule_addr),
            error::not_found(ENO_FEE_SCHEDULE),
        );
        fee_schedule_addr
    }

    // Tests

    #[test_only]
    use velor_framework::account;

    #[test(creator = @0x123)]
    fun test_init(
        creator: &signer,
    ) acquires FeeSchedule, FixedRateBiddingFee, FixedRateCommission, FixedRateListingFee, PercentageRateCommission {
        let creator_addr = signer::address_of(creator);
        account::create_account_for_test(creator_addr);
        let obj = init(creator, creator_addr, 0, 0, 1, 0);

        assert!(fee_address(obj) == creator_addr, 0);
        assert!(listing_fee(obj, 5) == 0, 0);
        assert!(bidding_fee(obj, 5) == 0, 0);
        assert!(commission(obj, 5) == 0, 0);

        set_fee_address(creator, obj, @0x0);
        set_fixed_rate_listing_fee(creator, obj, 5);
        set_fixed_rate_bidding_fee(creator, obj, 6);
        set_percentage_rate_commission(creator, obj, 10, 1);

        assert!(fee_address(obj) == @0x0, 0);
        assert!(listing_fee(obj, 5) == 5, 0);
        assert!(bidding_fee(obj, 5) == 6, 0);
        assert!(commission(obj, 20) == 2, 0);

        set_fixed_rate_commission(creator, obj, 8);
        assert!(commission(obj, 20) == 8, 0);
    }

    #[test(creator = @0x123)]
    fun test_empty_init(
        creator: &signer,
    ) acquires FeeSchedule, FixedRateBiddingFee, FixedRateCommission, FixedRateListingFee, PercentageRateCommission {
        let creator_addr = signer::address_of(creator);
        account::create_account_for_test(creator_addr);
        let (constructor_ref, _fee_schedule_signer) = empty_init(creator, creator_addr);
        let obj = object::object_from_constructor_ref(&constructor_ref);

        assert!(fee_address(obj) == creator_addr, 0);
        assert!(listing_fee(obj, 5) == 0, 0);
        assert!(bidding_fee(obj, 5) == 0, 0);
        assert!(commission(obj, 5) == 0, 0);

        set_fee_address(creator, obj, @0x0);
        set_fixed_rate_listing_fee(creator, obj, 5);
        set_fixed_rate_bidding_fee(creator, obj, 6);
        set_percentage_rate_commission(creator, obj, 10, 1);

        assert!(fee_address(obj) == @0x0, 0);
        assert!(listing_fee(obj, 5) == 5, 0);
        assert!(bidding_fee(obj, 5) == 6, 0);
        assert!(commission(obj, 20) == 2, 0);

        set_fixed_rate_commission(creator, obj, 8);
        assert!(commission(obj, 20) == 8, 0);
    }

    #[test(creator = @0x123, non_creator = @0x223)]
    #[expected_failure(abort_code = 0x50004, location = Self)]
    fun test_non_creator_fee_address(creator: &signer, non_creator: &signer) acquires FeeSchedule {
        let creator_addr = signer::address_of(creator);
        account::create_account_for_test(creator_addr);
        let obj = init(creator, creator_addr, 0, 0, 1, 0);
        set_fee_address(non_creator, obj, @0x0);
    }

    #[test(creator = @0x123, non_creator = @0x223)]
    #[expected_failure(abort_code = 0x50004, location = Self)]
    fun test_non_creator_fixed_listing(
        creator: &signer,
        non_creator: &signer,
    ) acquires FeeSchedule, FixedRateListingFee {
        let creator_addr = signer::address_of(creator);
        account::create_account_for_test(creator_addr);
        let obj = init(creator, creator_addr, 0, 0, 1, 0);
        set_fixed_rate_listing_fee(non_creator, obj, 5);
    }

    #[test(creator = @0x123, non_creator = @0x223)]
    #[expected_failure(abort_code = 0x50004, location = Self)]
    fun test_non_creator_fixed_bidding(
        creator: &signer,
        non_creator: &signer,
    ) acquires FeeSchedule, FixedRateBiddingFee {
        let creator_addr = signer::address_of(creator);
        account::create_account_for_test(creator_addr);
        let obj = init(creator, creator_addr, 0, 0, 1, 0);
        set_fixed_rate_bidding_fee(non_creator, obj, 6);
    }

    #[test(creator = @0x123, non_creator = @0x223)]
    #[expected_failure(abort_code = 0x50004, location = Self)]
    fun test_non_creator_percentage_commission(
        creator: &signer,
        non_creator: &signer,
    ) acquires FeeSchedule, FixedRateCommission, PercentageRateCommission {
        let creator_addr = signer::address_of(creator);
        account::create_account_for_test(creator_addr);
        let obj = init(creator, creator_addr, 0, 0, 1, 0);
        set_percentage_rate_commission(non_creator, obj, 10, 1);
    }

    #[test(creator = @0x123, non_creator = @0x223)]
    #[expected_failure(abort_code = 0x50004, location = Self)]
    fun test_non_creator_fixed_commission(
        creator: &signer,
        non_creator: &signer,
    ) acquires FeeSchedule, FixedRateCommission, PercentageRateCommission {
        let creator_addr = signer::address_of(creator);
        account::create_account_for_test(creator_addr);
        let obj = init(creator, creator_addr, 0, 0, 1, 0);
        set_fixed_rate_commission(non_creator, obj, 8);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x20002, location = Self)]
    fun test_init_zero_denominator_percentage_commission(creator: &signer) {
        let creator_addr = signer::address_of(creator);
        account::create_account_for_test(creator_addr);
        init(creator, creator_addr, 0, 0, 0, 0);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x20002, location = Self)]
    fun test_set_zero_denominator_percentage_commission(
        creator: &signer,
    ) acquires FeeSchedule, FixedRateCommission, PercentageRateCommission {
        let creator_addr = signer::address_of(creator);
        account::create_account_for_test(creator_addr);
        let obj = init(creator, creator_addr, 0, 0, 1, 0);
        set_percentage_rate_commission(creator, obj, 0, 0);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x10003, location = Self)]
    fun test_init_too_big_percentage_commission(creator: &signer) {
        let creator_addr = signer::address_of(creator);
        account::create_account_for_test(creator_addr);
        init(creator, creator_addr, 0, 0, 1, 2);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x10003, location = Self)]
    fun test_set_too_big_percentage_commission(
        creator: &signer,
    ) acquires FeeSchedule, FixedRateCommission, PercentageRateCommission {
        let creator_addr = signer::address_of(creator);
        account::create_account_for_test(creator_addr);
        let obj = init(creator, creator_addr, 0, 0, 1, 0);
        set_percentage_rate_commission(creator, obj, 1, 2);
    }
}
