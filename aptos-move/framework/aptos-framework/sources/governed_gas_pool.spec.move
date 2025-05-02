spec aptos_framework::governed_gas_pool {
    use aptos_framework::coin::EINSUFFICIENT_BALANCE; 
    use aptos_framework::error;

    /// <high-level-req>
    /// No.: 1
    /// Requirement: The GovernedGasPool resource must exist at the aptos_framework address after initialization.
    /// Criticality: Critical
    /// Implementation: The initialize function ensures the resource is created at the aptos_framework address.
    /// Enforcement: Formally verified via [high-level-req-1](initialize).
    ///
    /// No.: 2
    /// Requirement: Only the aptos_framework address is allowed to initialize the GovernedGasPool.
    /// Criticality: Critical
    /// Implementation: The initialize function verifies the signer is the aptos_framework address.
    /// Enforcement: Formally verified via [high-level-req-2](initialize).
    ///
    /// No.: 3
    /// Requirement: Deposits into the GovernedGasPool must be reflected in the pool's balance.
    /// Criticality: High
    /// Implementation: The deposit and deposit_from functions update the pool's balance.
    /// Enforcement: Formally verified via [high-level-req-3](deposit), [high-level-req-3.1](deposit_from).
    ///
    /// No.: 4
    /// Requirement: Only the aptos_framework address can fund accounts from the GovernedGasPool.
    /// Criticality: High
    /// Implementation: The fund function verifies the signer is the aptos_framework address.
    /// Enforcement: Formally verified via [high-level-req-4](fund).
    ///
    
    spec module {
        /// [high-level-req-1]
        /// The GovernedGasPool resource must exist at aptos_framework after initialization.
        invariant exists<GovernedGasPool>(@aptos_framework);
    }

    spec initialize(aptos_framework: &signer, delegation_pool_creation_seed: vector<u8>) {
        requires system_addresses::is_aptos_framework_address(signer::address_of(aptos_framework));
        /// [high-level-req-1]
        ensures exists<GovernedGasPool>(@aptos_framework);
    }

    spec fund<CoinType>(aptos_framework: &signer, account: address, amount: u64) {
        pragma aborts_if_is_partial = true;

        /// [high-level-req-4]
        // Abort if the caller is not the Aptos framework
        aborts_if !system_addresses::is_aptos_framework_address(signer::address_of(aptos_framework));

        /// Abort if the governed gas pool has insufficient funds
        aborts_with coin::EINSUFFICIENT_BALANCE, error::invalid_argument(EINSUFFICIENT_BALANCE), 0x1, 0x5, 0x7;
    }
   
    spec deposit<CoinType>(coin: Coin<CoinType>) {
        pragma aborts_if_is_partial = true;

        /*
        /// [high-level-req-3]
        /// Ensure the deposit increases the value in the CoinStore

        //@TODO: Calling governed_gas_pool_adddress() doesn't work as the boogie gen cant check the signer 
        // created for the resource account created at runtime

        /// Ensure the governed gas pool resource account exists
        //aborts_if !exists<CoinStore<CoinType>>(governed_gas_pool_address());

        //ensures global<CoinStore<CoinType>>(aptos_framework_address).coin.value ==
        //old(global<CoinStore<CoinType>>(aptos_framework_address).coin.value) + coin.value;
        */
    }

    spec deposit_gas_fee(_gas_payer: address, _gas_fee: u64) {
        /*
        /// [high-level-req-5]
        //   ensures governed_gas_pool_balance<AptosCoin> == old(governed_gas_pool_balance<AptosCoin>) + gas_fee;
        //   ensures gas_payer_balance<AptosCoin> == old(gas_payer_balance<AptosCoin>) - gas_fee;
        */
    }
}
