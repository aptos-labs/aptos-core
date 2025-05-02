module aptos_framework::governed_gas_pool {

    friend aptos_framework::transaction_validation;

    use std::vector;
    use aptos_framework::account::{Self, SignerCapability, create_signer_with_capability};
    use aptos_framework::system_addresses::{Self};
    // use aptos_framework::primary_fungible_store::{Self};
    use aptos_framework::fungible_asset::{Self};
    use aptos_framework::object::{Self};
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin::{Self, Coin};
    use std::features;
    use aptos_framework::signer;
    use aptos_framework::aptos_account::Self;
    #[test_only]
    use aptos_framework::coin::{BurnCapability, MintCapability};
    #[test_only]
    use aptos_framework::fungible_asset::BurnRef;
    #[test_only]
    use aptos_framework::aptos_coin::Self;

    const MODULE_SALT: vector<u8> = b"aptos_framework::governed_gas_pool";

    /// The Governed Gas Pool
    /// Internally, this is a simply wrapper around a resource account. 
    struct GovernedGasPool has key {
        /// The signer capability of the resource account.
        signer_capability: SignerCapability,
    }

    /// Address of APT Primary Fungible Store
    inline fun primary_fungible_store_address(account: address): address {
        object::create_user_derived_object_address(account, @aptos_fungible_asset)
    }

    /// Create the seed to derive the resource account address.
    fun create_resource_account_seed(
        delegation_pool_creation_seed: vector<u8>,
    ): vector<u8> {
        let seed = vector::empty<u8>();
        // include module salt (before any subseeds) to avoid conflicts with other modules creating resource accounts
        vector::append(&mut seed, MODULE_SALT);
        // include an additional salt in case the same resource account has already been created
        vector::append(&mut seed, delegation_pool_creation_seed);
        seed
    }

    /// Initializes the governed gas pool around a resource account creation seed. 
    /// @param aptos_framework The signer of the aptos_framework module.
    /// @param delegation_pool_creation_seed The seed to be used to create the resource account hosting the delegation pool.
    public fun initialize(
        aptos_framework: &signer,
        delegation_pool_creation_seed: vector<u8>,
    ) {
        system_addresses::assert_aptos_framework(aptos_framework);

        // return if the governed gas pool has already been initialized
        if (exists<GovernedGasPool>(signer::address_of(aptos_framework))) {
            return
        };

        // generate a seed to be used to create the resource account hosting the delegation pool
        let seed = create_resource_account_seed(delegation_pool_creation_seed);

        let (governed_gas_pool_signer, governed_gas_pool_signer_cap) = account::create_resource_account(aptos_framework, seed);

        // register apt
        aptos_account::register_apt(&governed_gas_pool_signer);

        move_to(aptos_framework, GovernedGasPool{
            signer_capability: governed_gas_pool_signer_cap,
        });
    }

    /// Initialize the governed gas pool as a module
    /// @param aptos_framework The signer of the aptos_framework module.
    fun init_module(aptos_framework: &signer) {
        // Initialize the governed gas pool
        let seed : vector<u8> = b"aptos_framework::governed_gas_pool";
        initialize(aptos_framework, seed);
    }

    /// Borrows the signer of the governed gas pool.
    /// @return The signer of the governed gas pool.
    fun governed_gas_signer(): signer acquires GovernedGasPool {
        let signer_cap = &borrow_global<GovernedGasPool>(@aptos_framework).signer_capability;
        create_signer_with_capability(signer_cap)
    }

    #[view]
    /// Gets the address of the governed gas pool.
    /// @return The address of the governed gas pool.
    public fun governed_gas_pool_address(): address acquires GovernedGasPool {
        signer::address_of(&governed_gas_signer())
    }

    /// Funds the destination account with a given amount of coin.
    /// @param account The account to be funded.
    /// @param amount The amount of coin to be funded.
    public fun fund<CoinType>(aptos_framework: &signer, account: address, amount: u64) acquires GovernedGasPool {
        // Check that the Aptos framework is the caller
        // This is what ensures that funding can only be done by the Aptos framework,
        // i.e., via a governance proposal.
        system_addresses::assert_aptos_framework(aptos_framework);
        let governed_gas_signer = &governed_gas_signer();
        coin::deposit(account, coin::withdraw<CoinType>(governed_gas_signer, amount));
    }

    /// Deposits some coin into the governed gas pool.
    /// @param coin The coin to be deposited.
    fun deposit<CoinType>(coin: Coin<CoinType>) acquires GovernedGasPool {
        let governed_gas_pool_address = governed_gas_pool_address();
        coin::deposit(governed_gas_pool_address, coin);
    }

    /// Deposits some coin from an account to the governed gas pool.
    /// @param account The account from which the coin is to be deposited.
    /// @param amount The amount of coin to be deposited.
    fun deposit_from<CoinType>(account: address, amount: u64) acquires GovernedGasPool {
       deposit(coin::withdraw_from<CoinType>(account, amount));
    }

    /// Deposits some FA from the fungible store. 
    /// @param aptos_framework The signer of the aptos_framework module.
    /// @param account The account from which the FA is to be deposited.
    /// @param amount The amount of FA to be deposited.
    fun deposit_from_fungible_store(account: address, amount: u64) acquires GovernedGasPool {
        if (amount > 0){
            // compute the governed gas pool store address
            let governed_gas_pool_address = governed_gas_pool_address();
            let governed_gas_pool_store_address = primary_fungible_store_address(governed_gas_pool_address);

            // compute the account store address
            let account_store_address = primary_fungible_store_address(account);
            fungible_asset::deposit_internal( 
                governed_gas_pool_store_address,
                fungible_asset::withdraw_internal(
                    account_store_address,
                    amount
                )
            );
        }
    }

    /// Deposits gas fees into the governed gas pool.
    /// @param gas_payer The address of the account that paid the gas fees.
    /// @param gas_fee The amount of gas fees to be deposited.
    public fun deposit_gas_fee(_gas_payer: address, _gas_fee: u64) acquires GovernedGasPool {
        // get the sender to preserve the signature but do nothing
        governed_gas_pool_address();
    }

    /// Deposits gas fees into the governed gas pool.
    /// @param gas_payer The address of the account that paid the gas fees.
    /// @param gas_fee The amount of gas fees to be deposited.
    public(friend) fun deposit_gas_fee_v2(gas_payer: address, gas_fee: u64) acquires GovernedGasPool {
       if (features::operations_default_to_fa_apt_store_enabled()) {
            deposit_from_fungible_store(gas_payer, gas_fee);
        } else {
            deposit_from<AptosCoin>(gas_payer, gas_fee);
        };
    }

    #[view]
    /// Gets the balance of a specified coin type in the governed gas pool.
    /// @return The balance of the coin in the pool.
    public fun get_balance<CoinType>(): u64 acquires GovernedGasPool {
        let pool_address = governed_gas_pool_address();
        coin::balance<CoinType>(pool_address)
    }

    #[test_only]
    /// The AptosCoin mint capability
    struct AptosCoinMintCapability has key {
        mint_cap: MintCapability<AptosCoin>,
    }

    #[test_only]
    /// The AptosCoin burn capability
    struct AptosCoinBurnCapability has key {
        burn_cap: BurnCapability<AptosCoin>,
    }

    #[test_only]
    /// The AptosFA burn capabilities
    struct AptosFABurnCapabilities has key {
        burn_ref: BurnRef,
    }


    #[test_only]
    /// Stores the mint capability for AptosCoin.
    ///
    /// @param aptos_framework The signer representing the Aptos framework.
    /// @param mint_cap The mint capability for AptosCoin.
    public fun store_aptos_coin_mint_cap(aptos_framework: &signer, mint_cap: MintCapability<AptosCoin>) {
        system_addresses::assert_aptos_framework(aptos_framework);
        move_to(aptos_framework, AptosCoinMintCapability { mint_cap })
    }

    #[test_only]
    /// Stores the burn capability for AptosCoin, converting to a fungible asset reference if the feature is enabled.
    ///
    /// @param aptos_framework The signer representing the Aptos framework.
    /// @param burn_cap The burn capability for AptosCoin.
    public fun store_aptos_coin_burn_cap(aptos_framework: &signer, burn_cap: BurnCapability<AptosCoin>) {
        system_addresses::assert_aptos_framework(aptos_framework);
        if (features::operations_default_to_fa_apt_store_enabled()) {
            let burn_ref = coin::convert_and_take_paired_burn_ref(burn_cap);
            move_to(aptos_framework, AptosFABurnCapabilities { burn_ref });
        } else {
            move_to(aptos_framework, AptosCoinBurnCapability { burn_cap })
        }
    }

    #[test_only]
    /// Initializes the governed gas pool around a fixed creation seed for testing
    ///
    /// @param aptos_framework The signer of the aptos_framework module.
    public fun initialize_for_test(
        aptos_framework: &signer,
    ) {

        // initialize the AptosCoin module
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        
        // Initialize the governed gas pool
        let seed : vector<u8> = b"test";
        initialize(aptos_framework, seed);

        // add the mint capability to the governed gas pool
        store_aptos_coin_mint_cap(aptos_framework, mint_cap);
        store_aptos_coin_burn_cap(aptos_framework, burn_cap);

    }

    #[test_only]
    /// Mints some coin to an account for testing purposes.
    ///
    /// @param account The account to which the coin is to be minted.
    /// @param amount The amount of coin to be minted.
    public fun mint_for_test(account: address, amount: u64) acquires AptosCoinMintCapability {
         coin::deposit(account, coin::mint(
            amount,
            &borrow_global<AptosCoinMintCapability>(@aptos_framework).mint_cap
        ));
    }

    #[test(aptos_framework = @aptos_framework, depositor = @0xdddd)]
    /// Deposits some coin into the governed gas pool.
    ///
    /// @param aptos_framework is the signer of the aptos_framework module.
    fun test_governed_gas_pool_deposit(aptos_framework: &signer, depositor: &signer) acquires GovernedGasPool, AptosCoinMintCapability {
       
        // initialize the modules
        initialize_for_test(aptos_framework);
    
        // create the depositor account and fund it
        aptos_account::create_account(signer::address_of(depositor));
        mint_for_test(signer::address_of(depositor), 1000);

        // get the balances for the depositor and the governed gas pool
        let depositor_balance = coin::balance<AptosCoin>(signer::address_of(depositor));
        let governed_gas_pool_balance = coin::balance<AptosCoin>(governed_gas_pool_address());

        // deposit some coin into the governed gas pool
        deposit_from<AptosCoin>(signer::address_of(depositor), 100);

        // check the balances after the deposit
        assert!(coin::balance<AptosCoin>(signer::address_of(depositor)) == depositor_balance - 100, 1);
        assert!(coin::balance<AptosCoin>(governed_gas_pool_address()) == governed_gas_pool_balance + 100, 2);
    
    }

    #[test(aptos_framework = @aptos_framework, depositor = @0xdddd)]
    /// Deposits some coin from an account to the governed gas pool as gas fees.
    ///
    /// @param aptos_framework is the signer of the aptos_framework module.
    /// @param depositor is the signer of the account from which the coin is to be deposited.
    fun test_governed_gas_pool_deposit_gas_fee(aptos_framework: &signer, depositor: &signer) acquires GovernedGasPool, AptosCoinMintCapability {
       
        // initialize the modules
        initialize_for_test(aptos_framework);
    
        // create the depositor account and fund it
        aptos_account::create_account(signer::address_of(depositor));
        mint_for_test(signer::address_of(depositor), 1000);

        // get the balances for the depositor and the governed gas pool
        let depositor_balance = coin::balance<AptosCoin>(signer::address_of(depositor));
        let governed_gas_pool_balance = coin::balance<AptosCoin>(governed_gas_pool_address());

        // deposit some coin into the governed gas pool as gas fees
        deposit_gas_fee_v2(signer::address_of(depositor), 100);

        // check the balances after the deposit
        assert!(coin::balance<AptosCoin>(signer::address_of(depositor)) == depositor_balance - 100, 1);
        assert!(coin::balance<AptosCoin>(governed_gas_pool_address()) == governed_gas_pool_balance + 100, 2);
    
    }

    #[test(aptos_framework = @aptos_framework)]
    /// Test for the get_balance view method.
    fun test_governed_gas_pool_get_balance(aptos_framework: &signer) acquires GovernedGasPool, AptosCoinMintCapability {
       
        // initialize the modules
        initialize_for_test(aptos_framework);

        // fund the governed gas pool
        let governed_gas_pool_address = governed_gas_pool_address();
        mint_for_test(governed_gas_pool_address, 1000);

        // assert the balance is correct
        assert!(get_balance<AptosCoin>() == 1000, 1);
    }

    #[test(aptos_framework = @aptos_framework, depositor = @0xdddd, beneficiary = @0xbbbb)]
    /// Funds the destination account with a given amount of coin.
    ///
    /// @param aptos_framework is the signer of the aptos_framework module.
    /// @param depositor is the signer of the account from which the coin is to be funded.
    /// @param beneficiary is the address of the account to be funded.
    fun test_governed_gas_pool_fund(aptos_framework: &signer, depositor: &signer, beneficiary: &signer) acquires GovernedGasPool, AptosCoinMintCapability {
       
        // initialize the modules
        initialize_for_test(aptos_framework);
    
        // create the depositor account and fund it
        aptos_account::create_account(signer::address_of(depositor));
        mint_for_test(signer::address_of(depositor), 1000);

        // get the balances for the depositor and the governed gas pool
        let depositor_balance = coin::balance<AptosCoin>(signer::address_of(depositor));
        let governed_gas_pool_balance = coin::balance<AptosCoin>(governed_gas_pool_address());

        // collect gas fees from the depositor
        deposit_gas_fee_v2(signer::address_of(depositor), 100);

        // check the balances after the deposit
        assert!(coin::balance<AptosCoin>(signer::address_of(depositor)) == depositor_balance - 100, 1);
        assert!(coin::balance<AptosCoin>(governed_gas_pool_address()) == governed_gas_pool_balance + 100, 2);

        // ensure the beneficiary account has registered with the AptosCoin module
        aptos_account::create_account(signer::address_of(beneficiary));
        aptos_account::register_apt(beneficiary);

        // fund the beneficiary account
        fund<AptosCoin>(aptos_framework, signer::address_of(beneficiary), 100);

        // check the balances after the funding
        assert!(coin::balance<AptosCoin>(governed_gas_pool_address()) == governed_gas_pool_balance, 3);
        assert!(coin::balance<AptosCoin>(signer::address_of(beneficiary)) == 100, 4);
    
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_initialize_is_idempotent(aptos_framework: &signer) {
        // initialize the governed gas pool
        initialize_for_test(aptos_framework);
        // initialize the governed gas pool again, no abort
        initialize(aptos_framework, vector::empty<u8>());
    }
}
