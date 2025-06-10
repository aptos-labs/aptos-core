/// Defines the charges associated with using a marketplace, namely:
/// * Commission, the units transferred to the marketplace upon sale.
module open_marketplace::fee_schedule {
    use std::error;
    use std::signer;
    use std::string::{Self, String};
    use aptos_std::math64;

    use aptos_framework::event;
    use aptos_framework::object::{Self, ConstructorRef, ExtendRef, Object};

    /// FeeSchedule does not exist.
    const E_NO_FEE_SCHEDULE: u64 = 1;
    /// The denominator in a fraction cannot be zero.
    const EDENOMINATOR_IS_ZERO: u64 = 2;
    /// The value represented by a fraction cannot be greater than 1.
    const EEXCEEDS_MAXIMUM: u64 = 3;
    /// The passed in signer is not the owner of the marketplace.
    const ENOT_OWNER: u64 = 4;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Defines marketplace fees
    struct FeeSchedule has key {
        /// Address to send fees to
        fee_address: address,
        /// Ref for changing the configuration of the marketplace
        extend_ref: ExtendRef,
        /// Commission configuration
        commission: Commission,
    }

    /// Enum representing different commission types
    enum Commission has copy, drop, store {
        /// No commission
        None,
        /// Fixed amount commission
        Fixed(u64),
        /// Percentage-based commission with denominator and numerator
        Percentage {
            denominator: u64,
            numerator: u64,
        },
    }

    #[event]
    /// Event representing a change to the marketplace configuration
    struct FeeConfigUpdated has drop, store {
        marketplace: address,
        /// The type info of the struct that was updated.
        updated_resource: String,
    }

    // Initializers

    /// Create a marketplace with a percentage commission.
    public entry fun init_percentage_entry(
        creator: &signer,
        fee_address: address,
        commission_denominator: u64,
        commission_numerator: u64,
    ) {
        init_percentage(
            creator,
            fee_address,
            commission_denominator,
            commission_numerator,
        );
    }

    /// Create a marketplace with a fixed commission.
    public entry fun init_fixed_entry(
        creator: &signer,
        fee_address: address,
        fixed_commission: u64,
    ) {
        init_fixed(
            creator,
            fee_address,
            fixed_commission,
        );
    }

    /// Initialize a fee schedule with a fixed commission
    public fun init_fixed(
        creator: &signer,
        fee_address: address,
        fixed_commission: u64,
    ): Object<FeeSchedule> {
        let commission = if (fixed_commission > 0) { 
            Commission::Fixed(fixed_commission) 
        } else { 
            Commission::None 
        };
        
        init_internal(creator, fee_address, commission)
    }

    /// Initialize a fee schedule with a percentage commission
    public fun init_percentage(
        creator: &signer,
        fee_address: address,
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

        let commission = Commission::Percentage {
            denominator: commission_denominator,
            numerator: commission_numerator,
        };
        
        init_internal(creator, fee_address, commission)
    }

    /// Internal helper function for initializing a fee schedule with any commission type
    inline fun init_internal(
        creator: &signer,
        fee_address: address,
        commission: Commission,
    ): Object<FeeSchedule> {
        let constructor_ref = object::create_object_from_account(creator);
        let extend_ref = object::generate_extend_ref(&constructor_ref);
        let fee_schedule_signer = object::generate_signer(&constructor_ref);

        let marketplace = FeeSchedule {
            fee_address,
            extend_ref,
            commission,
        };
        move_to(&fee_schedule_signer, marketplace);
        
        object::object_from_constructor_ref(&constructor_ref)
    }

    /// Create a marketplace fee schedule with zero commission.
    public entry fun init_zero_commission(creator: &signer, fee_address: address) {
        init_zero_commission_internal(creator, fee_address);
    }
    
    inline fun init_zero_commission_internal(
        creator: &signer,
        fee_address: address,
    ): (ConstructorRef, signer) {
        let constructor_ref = object::create_object_from_account(creator);
        let extend_ref = object::generate_extend_ref(&constructor_ref);
        let fee_schedule_signer = object::generate_signer(&constructor_ref);

        let marketplace = FeeSchedule {
            fee_address,
            extend_ref,
            commission: Commission::None,
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
        event::emit(FeeConfigUpdated { marketplace: fee_schedule_addr, updated_resource });
    }

    /// Remove any existing commission and set a fixed rate commission, or none if set to 0.
    public entry fun set_fixed_rate_commission(
        creator: &signer,
        marketplace: Object<FeeSchedule>,
        commission: u64,
    ) acquires FeeSchedule {
        let fee_schedule_addr = assert_exists_internal(&marketplace);
        assert!(
            object::is_owner(marketplace, signer::address_of(creator)),
            error::permission_denied(ENOT_OWNER),
        );
        let fee_schedule_obj = borrow_global_mut<FeeSchedule>(fee_schedule_addr);
        fee_schedule_obj.commission = if (commission > 0) { Commission::Fixed(commission) } else { Commission::None };
        
        let updated_resource = string::utf8(b"commission");
        event::emit(FeeConfigUpdated { marketplace: fee_schedule_addr, updated_resource });
    }

    /// Remove any existing commission and set a percentage rate commission.
    public entry fun set_percentage_rate_commission(
        creator: &signer,
        marketplace: Object<FeeSchedule>,
        denominator: u64,
        numerator: u64,
    ) acquires FeeSchedule {
        assert!(
            numerator <= denominator,
            error::invalid_argument(EEXCEEDS_MAXIMUM),
        );
        assert!(
            denominator != 0,
            error::out_of_range(EDENOMINATOR_IS_ZERO),
        );

        let fee_schedule_addr = assert_exists_internal(&marketplace);
        assert!(
            object::is_owner(marketplace, signer::address_of(creator)),
            error::permission_denied(ENOT_OWNER),
        );
        let fee_schedule_obj = borrow_global_mut<FeeSchedule>(fee_schedule_addr);
        fee_schedule_obj.commission = Commission::Percentage { denominator, numerator };
        
        let updated_resource = string::utf8(b"commission");
        event::emit(FeeConfigUpdated { marketplace: fee_schedule_addr, updated_resource });
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
    public fun commission(
        marketplace: Object<FeeSchedule>,
        price: u64,
    ): u64 acquires FeeSchedule {
        let fee_schedule_addr = assert_exists_internal(&marketplace);
        let fee_schedule_obj = borrow_global<FeeSchedule>(fee_schedule_addr);
        
        match (fee_schedule_obj.commission) {
            Commission::None => 0,
            Commission::Fixed(fee) => fee,
            Commission::Percentage { denominator, numerator } => math64::mul_div(price, numerator, denominator),
        }
    }

    public fun assert_exists(marketplace: &Object<FeeSchedule>) {
        assert_exists_internal(marketplace);
    }

    inline fun assert_exists_internal(marketplace: &Object<FeeSchedule>): address {
        let fee_schedule_addr = object::object_address(marketplace);
        assert!(
            exists<FeeSchedule>(fee_schedule_addr),
            error::not_found(E_NO_FEE_SCHEDULE),
        );
        fee_schedule_addr
    }

    // Tests

    #[test_only]
    use aptos_framework::account;
    #[test_only]
    use aptos_framework::fungible_asset::{Self, Metadata};
    #[test_only]
    use aptos_framework::primary_fungible_store;
    #[test_only]
    use aptos_framework::coin;
    #[test_only]
    use aptos_framework::aptos_coin::AptosCoin;
    #[test_only]
    use aptos_framework::coin::MintCapability;

    #[test_only]
    const DEFAULT_STARTING_BALANCE: u64 = 10000;

    #[test_only]
    fun get_mint_cap(aptos_framework: &signer): MintCapability<AptosCoin> {
        let (burn_cap, freeze_cap, mint_cap) =
            coin::initialize<AptosCoin>(
                aptos_framework,
                string::utf8(b"TC"),
                string::utf8(b"TC"),
                8,
                false
            );
        coin::destroy_freeze_cap(freeze_cap);
        coin::destroy_burn_cap(burn_cap);
        mint_cap
    }

    #[test_only]
    fun create_test_account(
        mint_cap: &MintCapability<AptosCoin>, account: &signer
    ) {
        account::create_account_for_test(signer::address_of(account));
        coin::register<AptosCoin>(account);
        let coins = coin::mint<AptosCoin>(DEFAULT_STARTING_BALANCE, mint_cap);
        coin::deposit(signer::address_of(account), coins);
    }

    #[test_only]
    /// Call this at the start of each test.
    fun initialize(
        creator: &signer,
        aptos_framework: &signer
    ) {
        let mint_cap = get_mint_cap(aptos_framework);
        coin::create_coin_conversion_map(aptos_framework);
        coin::create_pairing<AptosCoin>(aptos_framework);
        create_test_account(&mint_cap, creator);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test_only]
    /// Empty struct for testing
    struct AptosDummyMetadata has key {}

    #[test_only]
    /// Helper function to get FA metadata from AptosCoin
    fun get_aptos_coin_metadata(creator: &signer): Object<Metadata> {
        let creator_addr = signer::address_of(creator);
        
        // Create FA metadata from real AptosCoin
        let coin = coin::withdraw<AptosCoin>(creator, 1000);
        let fa = coin::coin_to_fungible_asset(coin);
        let fa_metadata = fungible_asset::metadata_from_asset(&fa);
        
        // Deposit the fungible asset back to avoid the drop error
        primary_fungible_store::deposit(creator_addr, fa);
        
        fa_metadata
    }

    #[test_only]
    /// Helper function to initialize a fee schedule object for testing
    fun init_test_fee_schedule(
        creator: &signer,
    ): (Object<FeeSchedule>, address) {
        let creator_addr = signer::address_of(creator);
        
        let obj = init_percentage(creator, creator_addr, 1, 0);
        (obj, creator_addr)
    }

    #[test_only]
    /// Helper function to initialize a fee schedule object with fixed commission for testing
    fun init_test_fixed_fee_schedule(
        creator: &signer,
        fixed_commission: u64,
    ): (Object<FeeSchedule>, address) {
        let creator_addr = signer::address_of(creator);
        
        let obj = init_fixed(
            creator, 
            creator_addr, 
            fixed_commission
        );
        (obj, creator_addr)
    }

    #[test(creator = @0x123, aptos_framework = @aptos_framework)]
    fun test_init_percentage(
        creator: &signer,
        aptos_framework: &signer,
    ) acquires FeeSchedule {
        initialize(
            creator,
            aptos_framework,
        );
        let creator_addr = signer::address_of(creator);
        
        let obj = init_percentage(
            creator, 
            creator_addr, 
            1, 
            0
        );

        assert!(fee_address(obj) == creator_addr, 0);
        assert!(commission(obj, 5) == 0, 0);

        set_fee_address(creator, obj, @0x0);
        set_percentage_rate_commission(creator, obj, 10, 1);

        assert!(fee_address(obj) == @0x0, 0);
        assert!(commission(obj, 20) == 2, 0);

        set_fixed_rate_commission(creator, obj, 8);
        assert!(commission(obj, 20) == 8, 0);
    }

    #[test(creator = @0x123, aptos_framework = @aptos_framework)]
    fun test_init_fixed(
        creator: &signer,
        aptos_framework: &signer,
    ) acquires FeeSchedule {
        initialize(
            creator,
            aptos_framework,
        );
        let creator_addr = signer::address_of(creator);
        
        let fixed_commission = 100; // Fixed 100 unit commission
        let obj = init_fixed(
            creator, 
            creator_addr, 
            fixed_commission
        );

        assert!(fee_address(obj) == creator_addr, 0);
        assert!(commission(obj, 50) == fixed_commission, 0); // Should be fixed regardless of price
        assert!(commission(obj, 5000) == fixed_commission, 0); // Should be fixed regardless of price

        // Test changing to percentage commission
        set_percentage_rate_commission(creator, obj, 10, 1);
        assert!(commission(obj, 20) == 2, 0);

        // Test changing back to fixed commission
        set_fixed_rate_commission(creator, obj, 8);
        assert!(commission(obj, 20) == 8, 0);
    }

    #[test(creator = @0x123, aptos_framework = @aptos_framework)]
    fun test_init_zero_commission(
        creator: &signer,
        aptos_framework: &signer,
    ) acquires FeeSchedule {
        initialize(
            creator,
            aptos_framework,
        );
        let creator_addr = signer::address_of(creator);
        
        
        let (constructor_ref, _fee_schedule_signer) = init_zero_commission_internal(creator, creator_addr);
        let obj = object::object_from_constructor_ref(&constructor_ref);

        assert!(fee_address(obj) == creator_addr, 0);
        assert!(commission(obj, 5) == 0, 0);

        set_fee_address(creator, obj, @0x0);
        set_percentage_rate_commission(creator, obj, 10, 1);

        assert!(fee_address(obj) == @0x0, 0);
        assert!(commission(obj, 20) == 2, 0);

        set_fixed_rate_commission(creator, obj, 8);
        assert!(commission(obj, 20) == 8, 0);
    }

    #[test(creator = @0x123, non_creator = @0x223, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x50004, location = Self)]
    fun test_non_creator_fee_address(
        creator: &signer, 
        non_creator: &signer,
        aptos_framework: &signer
    ) acquires FeeSchedule {
        initialize(
            creator,
            aptos_framework,
        );
        account::create_account_for_test(signer::address_of(non_creator));
        
        let (obj, _) = init_test_fee_schedule(creator);
        set_fee_address(non_creator, obj, @0x0);
    }

    #[test(creator = @0x123, non_creator = @0x223, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x50004, location = Self)]
    fun test_non_creator_percentage_commission(
        creator: &signer,
        non_creator: &signer,
        aptos_framework: &signer
    ) acquires FeeSchedule {
        initialize(
            creator,
            aptos_framework,
        );
        account::create_account_for_test(signer::address_of(non_creator));
        
        let (obj, _) = init_test_fee_schedule(creator);
        set_percentage_rate_commission(non_creator, obj, 10, 1);
    }

    #[test(creator = @0x123, non_creator = @0x223, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x50004, location = Self)]
    fun test_non_creator_fixed_commission(
        creator: &signer,
        non_creator: &signer,
        aptos_framework: &signer
    ) acquires FeeSchedule {
        initialize(
            creator,
            aptos_framework,
        );
        account::create_account_for_test(signer::address_of(non_creator));
        
        let (obj, _) = init_test_fee_schedule(creator);
        set_fixed_rate_commission(non_creator, obj, 8);
    }

    #[test(creator = @0x123, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x20002, location = Self)]
    fun test_init_zero_denominator_percentage_commission(
        creator: &signer,
        aptos_framework: &signer
    ) {
        initialize(
            creator,
            aptos_framework,
        );
        let creator_addr = signer::address_of(creator);
        
        init_percentage(creator, creator_addr, 0, 0);
    }

    #[test(creator = @0x123, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10003, location = Self)]
    fun test_init_too_big_percentage_commission(
        creator: &signer,
        aptos_framework: &signer
    ) {
        initialize(
            creator,
            aptos_framework,
        );
        let creator_addr = signer::address_of(creator);
        
        init_percentage(creator, creator_addr, 0, 2);
    }

    #[test(creator = @0x123, aptos_framework = @aptos_framework)]
    fun test_init_fixed_commission(
        creator: &signer,
        aptos_framework: &signer,
    ) acquires FeeSchedule {
        initialize(
            creator,
            aptos_framework,
        );
        
        let fixed_commission = 50;
        let (obj, creator_addr) = init_test_fixed_fee_schedule(creator, fixed_commission);

        assert!(fee_address(obj) == creator_addr, 0);
        assert!(commission(obj, 100) == fixed_commission, 0); // Should be fixed regardless of price
        assert!(commission(obj, 10000) == fixed_commission, 0); // Should be fixed regardless of price
    }

    #[test(creator = @0x123, aptos_framework = @aptos_framework)]
    fun test_init_zero_fixed_commission(
        creator: &signer,
        aptos_framework: &signer,
    ) acquires FeeSchedule {
        initialize(
            creator,
            aptos_framework,
        );
        
        let (obj, _) = init_test_fixed_fee_schedule(creator, 0);
        
        // Commission should be zero regardless of price
        assert!(commission(obj, 100) == 0, 0);
        assert!(commission(obj, 10000) == 0, 0);
        
        set_percentage_rate_commission(creator, obj, 10, 1);
        assert!(commission(obj, 100) == 10, 0); // Should now be 10% of 100
    }

    #[test(creator = @0x123, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x20002, location = Self)]
    fun test_set_zero_denominator_percentage_commission(
        creator: &signer,
        aptos_framework: &signer
    ) acquires FeeSchedule {
        initialize(
            creator,
            aptos_framework,
        );
        
        let (obj, _) = init_test_fee_schedule(creator);
        set_percentage_rate_commission(creator, obj, 0, 0);
    }

    #[test(creator = @0x123, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10003, location = Self)]
    fun test_set_too_big_percentage_commission(
        creator: &signer,
        aptos_framework: &signer
    ) acquires FeeSchedule {
        initialize(
            creator,
            aptos_framework,
        );
        let creator_addr = signer::address_of(creator);
        
        let obj = init_percentage(creator, creator_addr, 1, 0);
        set_percentage_rate_commission(creator, obj, 1, 2);
    }
}
