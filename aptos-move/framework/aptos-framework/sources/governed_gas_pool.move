/**
MIP-52: https://github.com/movementlabsxyz/MIP/pull/52

The Governed Gas Pool is a pool into which and when enabled all gas fees are deposited.

Non-view methods herein are only intended to be called by the aptos_framework, hence via a governance proposal.

The implementation provided is based on Aptos Lab's Delegation Pool implementation: https://github.com/aptos-labs/aptos-core/blob/7e0aaa2ad12759f6afd6bac04bc55c2ea8046676/aptos-move/framework/aptos-framework/sources/delegation_pool.move#L4
*/
module aptos_framework::governed_gas_pool {
    use std::vector;
    use aptos_framework::account::{Self, SignerCapability, create_signer_with_capability};
    use aptos_framework::system_addresses::{Self};
    use aptos_framework::primary_fungible_store::{Self};
    use aptos_framework::fungible_asset::{Self};
    use aptos_framework::object::{Self};
    use aptos_framework::aptos_coin::{Self, AptosCoin};
    use aptos_framework::coin::{Self, Coin};
    use std::features;
    use aptos_framework::signer;

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
        // generate a seed to be used to create the resource account hosting the delegation pool
        let seed = create_resource_account_seed(delegation_pool_creation_seed);

        let (governed_gas_pool_signer, governed_gas_pool_signer_cap) = account::create_resource_account(aptos_framework, seed);

        move_to(&governed_gas_pool_signer, GovernedGasPool{
            signer_capability: governed_gas_pool_signer_cap,
        });
    }

    /// Borrows the signer of the governed gas pool.
    /// @return The signer of the governed gas pool.
    fun governed_gas_signer(): signer  acquires GovernedGasPool {
        let signer_cap = &borrow_global<GovernedGasPool>(@aptos_framework).signer_capability;
        create_signer_with_capability(signer_cap)
    }

    /// Gets the address of the governed gas pool.
    /// @return The address of the governed gas pool.
    public fun governed_gas_pool_address(): address acquires GovernedGasPool {
        signer::address_of(&governed_gas_signer())
    }

    /// Funds the destination account with a given amount of coin.
    /// @param account The account to be funded.
    /// @param amount The amount of coin to be funded.
    public fun fund<CoinType>(aptos_framework: &signer, account: address, amount: u64) acquires GovernedGasPool {
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
    public fun deposit_gas_fee(gas_payer: address, gas_fee: u64) acquires GovernedGasPool {
        
        if (features::operations_default_to_fa_apt_store_enabled()) {
            deposit_from_fungible_store(gas_payer, gas_fee);
        } else {
            deposit_from<AptosCoin>(gas_payer, gas_fee);
        };

    }

    #[test]
    /// Initializes the governed gas pool around a fixed creation seed for testing
    /// @param aptos_framework The signer of the aptos_framework module.
    public fun initialize_for_testing(
        aptos_framework: &signer,
    ) {
        
        let seed : vector<u8> = b"test";
        initialize(aptos_framework, seed);

    }
    
}